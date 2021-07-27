#![allow(dead_code)]

use chain_impl_mockchain::certificate::VotePlanId;
use std::collections::HashMap;
use std::iter;
use std::ops::Range;
use thiserror::Error;

pub struct VoteCastCounter {
    register: Vec<WalletVoteCastPosition>,
}

impl VoteCastCounter {
    pub fn new(wallet_count: usize, vote_plans: Vec<(VotePlanId, u8)>) -> Self {
        Self {
            register: iter::from_fn(|| Some(WalletVoteCastPosition::new(vote_plans.clone())))
                .take(wallet_count)
                .collect(),
        }
    }

    pub fn available_to_send(&self) -> usize {
        self.register.iter().map(|x| x.available_to_send()).sum()
    }

    pub fn is_drained(&self) -> bool {
        self.available_to_send() == 0
    }

    pub fn advance_single(&mut self, wallet_idx: usize) -> Result<WalletVotesToCast, Error> {
        self.advance_batch(1, wallet_idx)
    }

    pub fn advance_batch(
        &mut self,
        mut requested_batch_size: usize,
        wallet_idx: usize,
    ) -> Result<WalletVotesToCast, Error> {
        let mut wallet_votes_to_cast = WalletVotesToCast::new();

        let vote_plan = self.register.get_mut(wallet_idx).unwrap();

        let votes_to_cast = vote_plan.advance_batch(requested_batch_size).map_err(|_| {
            Error::NoMoreRequestsToSentForLoad {
                wallet_idx,
                requested_batch_size,
            }
        })?;
        votes_to_cast.iter().for_each(|votes_to_cast| {
            requested_batch_size -= votes_to_cast.range.end - votes_to_cast.range.start
        });
        wallet_votes_to_cast.add(wallet_idx, votes_to_cast);

        Ok(wallet_votes_to_cast)
    }
}

pub struct WalletVoteCastPosition {
    register: Vec<VotePlanVoteCastPosition>,
}

impl WalletVoteCastPosition {
    pub fn new(vote_plans: Vec<(VotePlanId, u8)>) -> Self {
        Self {
            register: vote_plans
                .iter()
                .map(|(id, limit)| VotePlanVoteCastPosition::new(id.clone(), *limit))
                .collect(),
        }
    }

    pub fn has_next(&self) -> bool {
        !self.is_drained()
    }

    pub fn is_drained(&self) -> bool {
        self.register.iter().all(|x| x.is_drained())
    }

    pub fn advance_single_unsafe(&mut self) -> Result<Vec<VotesToCast>, Error> {
        self.advance_batch_unsafe(1)
    }

    pub fn can_send_next_batch(&self, requested_batch_size: usize) -> bool {
        self.available_to_send() > requested_batch_size
    }

    pub fn available_to_send(&self) -> usize {
        self.register.iter().map(|x| x.available_to_send()).sum()
    }

    pub fn advance_batch_unsafe(
        &mut self,
        mut requested_batch_size: usize,
    ) -> Result<Vec<VotesToCast>, Error> {
        if !self.can_send_next_batch(requested_batch_size) {
            return Err(Error::NoMoreRequestsToSentForWallet {
                requested_batch_size,
                available_to_send: self.available_to_send(),
            });
        }

        let mut votes_to_cast = Vec::new();

        for vote_plan in self.register.iter_mut().skip_while(|x| x.is_drained()) {
            if requested_batch_size == 0 {
                break;
            }

            let batch_size = std::cmp::min(requested_batch_size, vote_plan.available_to_send());
            requested_batch_size -= batch_size;
            votes_to_cast.push(vote_plan.advance_batch_unsafe(batch_size)?);
        }

        Ok(votes_to_cast)
    }

    pub fn advance_batch(
        &mut self,
        mut requested_batch_size: usize,
    ) -> Result<Vec<VotesToCast>, Error> {
        let mut votes_to_cast = Vec::new();
        for vote_plan in self.register.iter_mut().skip_while(|x| x.is_drained()) {
            if requested_batch_size == 0 {
                break;
            }

            let batch_size = std::cmp::min(requested_batch_size, vote_plan.available_to_send());
            requested_batch_size -= batch_size;
            votes_to_cast.push(vote_plan.advance_batch(batch_size));
        }
        Ok(votes_to_cast)
    }
}

#[derive(Debug)]
pub struct VotePlanVoteCastPosition {
    id: VotePlanId,
    limit: u8,
    current: u8,
}

impl VotePlanVoteCastPosition {
    pub fn new(id: VotePlanId, limit: u8) -> Self {
        Self {
            id,
            limit,
            current: 0,
        }
    }

    pub fn id(&self) -> VotePlanId {
        self.id.clone()
    }

    pub fn is_drained(&self) -> bool {
        !self.has_next()
    }
    pub fn has_next(&self) -> bool {
        self.can_send_next_batch(1)
    }

    pub fn can_send_next_batch(&self, requested_batch_size: usize) -> bool {
        self.available_to_send() >= requested_batch_size
    }

    pub fn available_to_send(&self) -> usize {
        (self.limit - self.current).into()
    }

    pub fn advance_single_unsafe(&mut self) -> Result<VotesToCast, Error> {
        self.advance_batch_unsafe(1)
    }
    pub fn advance_batch_unsafe(
        &mut self,
        requested_batch_size: usize,
    ) -> Result<VotesToCast, Error> {
        if !self.can_send_next_batch(requested_batch_size) {
            return Err(Error::NoMoreRequestsToSent {
                requested_batch_size,
                available_to_send: self.available_to_send(),
            });
        }

        let current_usize = self.current as usize;
        let output = VotesToCast {
            id: self.id.clone(),
            range: current_usize..(requested_batch_size + current_usize),
        };

        self.current += requested_batch_size as u8;

        Ok(output)
    }

    pub fn advance_batch(&mut self, requested_batch_size: usize) -> VotesToCast {
        let batch_size = std::cmp::min(self.available_to_send(), requested_batch_size);
        self.advance_batch_unsafe(batch_size).unwrap()
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct VotesToCast {
    id: VotePlanId,
    range: Range<usize>,
}

impl VotesToCast {
    pub fn new(id: VotePlanId, range: Range<usize>) -> Self {
        Self { id, range }
    }

    pub fn count(&self) -> usize {
        self.range.end - self.range.start
    }

    pub fn id(&self) -> VotePlanId {
        self.id.clone()
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletVotesToCast {
    votes: HashMap<usize, Vec<VotesToCast>>,
}

impl WalletVotesToCast {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
        }
    }

    pub fn add(&mut self, idx: usize, mut votes_to_cast: Vec<VotesToCast>) {
        if let Some(votes) = self.votes.get_mut(&idx) {
            votes.append(&mut votes_to_cast);
            return;
        }

        self.votes.insert(idx, votes_to_cast);
    }

    pub fn is_empty(&self) -> bool {
        self.count() != 0
    }

    pub fn count(&self) -> usize {
        self.votes
            .iter()
            .fold(0, |_, (_, votes)| votes.iter().map(|y| y.count()).sum())
    }

    pub fn votes(&self) -> HashMap<usize, Vec<VotesToCast>> {
        self.votes.clone()
    }

    pub fn next(&self, id: usize) -> VotesToCast {
        let votes = self.votes.get(&id).unwrap();
        votes.iter().next().unwrap().clone()
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(
        "no more requests to run requested: {requested_batch_size}, available: {available_to_send}"
    )]
    NoMoreRequestsToSentForWallet {
        requested_batch_size: usize,
        available_to_send: usize,
    },
    #[error("no more requests to run requested: {requested_batch_size}, wallet_idx: {wallet_idx}")]
    NoMoreRequestsToSentForLoad {
        wallet_idx: usize,
        requested_batch_size: usize,
    },
    #[error(
        "no more requests to run requested: {requested_batch_size}, available: {available_to_send}"
    )]
    NoMoreRequestsToSent {
        requested_batch_size: usize,
        available_to_send: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_impl_mockchain::testing::VoteTestGen;

    #[test]
    pub fn vote_plan_vote_cast_position_test() {
        let vote_plan_id = VoteTestGen::vote_plan().to_id();
        let limit = 255usize;
        let mut vote_plan_vote_cast_position =
            VotePlanVoteCastPosition::new(vote_plan_id.clone(), limit as u8);

        assert_eq!(vote_plan_id, vote_plan_vote_cast_position.id());
        assert!(!vote_plan_vote_cast_position.is_drained());
        assert!(vote_plan_vote_cast_position.has_next());
        assert!(vote_plan_vote_cast_position.can_send_next_batch(1));
        assert!(vote_plan_vote_cast_position.can_send_next_batch(limit));
        assert_eq!(limit, vote_plan_vote_cast_position.available_to_send());

        let votes_to_cast = vote_plan_vote_cast_position
            .advance_single_unsafe()
            .unwrap();
        assert_eq!(
            votes_to_cast,
            VotesToCast {
                id: vote_plan_id.clone(),
                range: 0..1
            }
        );

        let votes_to_cast = vote_plan_vote_cast_position
            .advance_batch_unsafe(10)
            .unwrap();
        assert_eq!(
            votes_to_cast,
            VotesToCast {
                id: vote_plan_id.clone(),
                range: 1..11
            }
        );

        assert!(vote_plan_vote_cast_position
            .advance_batch_unsafe(limit)
            .is_err());

        assert_eq!(
            vote_plan_vote_cast_position.advance_batch(limit),
            VotesToCast {
                id: vote_plan_id.clone(),
                range: 11..255
            }
        );

        assert!(vote_plan_vote_cast_position.is_drained());
        assert!(!vote_plan_vote_cast_position.has_next());
        assert!(!vote_plan_vote_cast_position.can_send_next_batch(1));
        assert_eq!(0, vote_plan_vote_cast_position.available_to_send());
    }

    #[test]
    pub fn vote_plan_vote_wallet_cast_position_test() {
        let vote_plan_id_1 = VoteTestGen::vote_plan().to_id();
        let vote_plan_id_2 = VoteTestGen::vote_plan().to_id();

        let limit_1 = 255usize;
        let limit_2 = 120usize;
        let mut vote_plan_vote_cast_position = WalletVoteCastPosition::new(vec![
            (vote_plan_id_1.clone(), limit_1 as u8),
            (vote_plan_id_2.clone(), limit_2 as u8),
        ]);

        assert!(!vote_plan_vote_cast_position.is_drained());
        assert!(vote_plan_vote_cast_position.has_next());
        assert!(vote_plan_vote_cast_position.can_send_next_batch(1));
        assert!(vote_plan_vote_cast_position.can_send_next_batch(limit_1 + limit_2 - 1));

        let votes_to_cast = vote_plan_vote_cast_position
            .advance_single_unsafe()
            .unwrap();
        assert_eq!(
            votes_to_cast,
            vec![VotesToCast {
                id: vote_plan_id_1.clone(),
                range: 0..1
            }]
        );

        let votes_to_cast = vote_plan_vote_cast_position
            .advance_batch_unsafe(300)
            .unwrap();
        assert_eq!(
            votes_to_cast,
            vec![
                VotesToCast {
                    id: vote_plan_id_1.clone(),
                    range: 1..255
                },
                VotesToCast {
                    id: vote_plan_id_2.clone(),
                    range: 0..46
                }
            ]
        );

        assert!(vote_plan_vote_cast_position
            .advance_batch_unsafe(300)
            .is_err());

        assert_eq!(
            vote_plan_vote_cast_position.advance_batch(90).unwrap(),
            vec![VotesToCast {
                id: vote_plan_id_2.clone(),
                range: 46..120
            }]
        );

        assert!(vote_plan_vote_cast_position.is_drained());
        assert!(!vote_plan_vote_cast_position.has_next());
        assert!(!vote_plan_vote_cast_position.can_send_next_batch(1));
        assert_eq!(0, vote_plan_vote_cast_position.available_to_send());
    }

    #[test]
    pub fn vote_cast_counter_advance_single_test() {
        let vote_plan_id_1 = VoteTestGen::vote_plan().to_id();
        let vote_plan_id_2 = VoteTestGen::vote_plan().to_id();

        let limit_1 = 255usize;
        let limit_2 = 120usize;
        let wallets = 2;
        let mut vote_cast_counter = VoteCastCounter::new(
            wallets,
            vec![
                (vote_plan_id_1.clone(), limit_1 as u8),
                (vote_plan_id_2.clone(), limit_2 as u8),
            ],
        );

        assert_eq!(
            vote_cast_counter.available_to_send(),
            wallets * (limit_1 + limit_2)
        );
        assert!(!vote_cast_counter.is_drained());

        for i in 0..wallets {
            for j in 0..375 {
                println!("{}{}", i, j);
                let mut expected = WalletVotesToCast::new();
                let expected_id = {
                    if j < limit_1 {
                        vote_plan_id_1.clone()
                    } else {
                        vote_plan_id_2.clone()
                    }
                };
                expected.add(
                    i,
                    vec![VotesToCast {
                        id: expected_id,
                        range: j % limit_1..j % limit_1 + 1,
                    }],
                );
                assert_eq!(vote_cast_counter.advance_single(i).unwrap(), expected);
            }
        }

        assert_eq!(vote_cast_counter.available_to_send(), 0);
        assert!(vote_cast_counter.is_drained());
    }

    #[test]
    pub fn vote_cast_counter_advance_batch_test() {
        let vote_plan_id_1 = VoteTestGen::vote_plan().to_id();
        let vote_plan_id_2 = VoteTestGen::vote_plan().to_id();

        let limit_1 = 70usize;
        let limit_2 = 40usize;
        let wallets = 1;

        let mut vote_cast_counter = VoteCastCounter::new(
            wallets,
            vec![
                (vote_plan_id_1.clone(), limit_1 as u8),
                (vote_plan_id_2.clone(), limit_2 as u8),
            ],
        );

        let mut expected_wallet_cast = WalletVotesToCast::new();
        expected_wallet_cast.add(
            0,
            vec![VotesToCast {
                id: vote_plan_id_1.clone(),
                range: 0..50,
            }],
        );

        assert_eq!(
            vote_cast_counter.advance_batch(50, 0).unwrap(),
            expected_wallet_cast
        );

        let mut expected_wallet_cast = WalletVotesToCast::new();
        expected_wallet_cast.add(
            0,
            vec![
                VotesToCast {
                    id: vote_plan_id_1.clone(),
                    range: 50..70,
                },
                VotesToCast {
                    id: vote_plan_id_2.clone(),
                    range: 0..30,
                },
            ],
        );

        assert_eq!(
            vote_cast_counter.advance_batch(50, 0).unwrap(),
            expected_wallet_cast
        );

        let mut expected_wallet_cast = WalletVotesToCast::new();
        expected_wallet_cast.add(
            0,
            vec![VotesToCast {
                id: vote_plan_id_2.clone(),
                range: 30..40,
            }],
        );

        assert_eq!(
            vote_cast_counter.advance_batch(50, 0).unwrap(),
            expected_wallet_cast
        );

        assert!(vote_cast_counter.advance_batch(50, 0).is_ok());
    }
}
