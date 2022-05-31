use crate::util_types::simple_hasher::Hasher;

use super::{
    addition_record::AdditionRecord, membership_proof::MembershipProof,
    removal_record::RemovalRecord,
};

pub trait MutatorSet<H>
where
    H: Hasher,
{
    /// Returns an empty mutator set
    fn default() -> Self;

    /**
     * prove
     * Generates a membership proof that will the valid when the item
     * is added to the mutator set.
     */
    fn prove(
        &self,
        item: &H::Digest,
        randomness: &H::Digest,
        store_bits: bool,
    ) -> MembershipProof<H>;
    fn verify(&self, item: &H::Digest, membership_proof: &MembershipProof<H>) -> bool;

    /// Generates an addition record from an item and explicit random-
    /// ness. The addition record is itself a commitment to the item,
    /// but tailored to adding the item to the mutator set in its
    /// current state.
    fn commit(&self, item: &H::Digest, randomness: &H::Digest) -> AdditionRecord<H>;

    /**
     * drop
     * Generates a removal record with which to update the set commitment.
     */
    fn drop(&self, item: &H::Digest, membership_proof: &MembershipProof<H>) -> RemovalRecord<H>;

    ///   add
    ///   Updates the set-commitment with an addition record. The new
    ///   commitment represents the set $S union {c}$ ,
    ///   where S is the set represented by the old
    ///   commitment and c is the commitment to the new item AKA the
    ///   *addition record*.
    fn add(&mut self, addition_record: &AdditionRecord<H>);

    /// remove
    /// Updates the mutator set so as to remove the item determined by
    /// its removal record.
    fn remove(&mut self, removal_record: &RemovalRecord<H>);
}
