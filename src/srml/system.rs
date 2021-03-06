//! Implements support for the srml_system module.
use crate::{
    error::Error,
    Client,
    XtBuilder,
};
use futures::future::{
    self,
    Future,
};
use parity_scale_codec::Codec;
use runtime_primitives::traits::{
    Bounded,
    CheckEqual,
    Hash,
    Header,
    MaybeDisplay,
    MaybeSerializeDebug,
    MaybeSerializeDebugButNotDeserialize,
    Member,
    SignedExtension,
    SimpleArithmetic,
    SimpleBitOps,
    StaticLookup,
};
use runtime_support::Parameter;
use serde::de::DeserializeOwned;
use srml_system::Event;
use substrate_primitives::Pair;

/// The subset of the `srml_system::Trait` that a client must implement.
pub trait System {
    /// Account index (aka nonce) type. This stores the number of previous
    /// transactions associated with a sender account.
    type Index: Parameter
        + Member
        + MaybeSerializeDebugButNotDeserialize
        + Default
        + MaybeDisplay
        + SimpleArithmetic
        + Copy;

    /// The block number type used by the runtime.
    type BlockNumber: Parameter
        + Member
        + MaybeSerializeDebug
        + MaybeDisplay
        + SimpleArithmetic
        + Default
        + Bounded
        + Copy
        + std::hash::Hash;

    /// The output of the `Hashing` function.
    type Hash: Parameter
        + Member
        + MaybeSerializeDebug
        + MaybeDisplay
        + SimpleBitOps
        + Default
        + Copy
        + CheckEqual
        + std::hash::Hash
        + AsRef<[u8]>
        + AsMut<[u8]>;

    /// The hashing system (algorithm) being used in the runtime (e.g. Blake2).
    type Hashing: Hash<Output = Self::Hash>;

    /// The user account identifier type for the runtime.
    type AccountId: Parameter
        + Member
        + MaybeSerializeDebug
        + MaybeDisplay
        + Ord
        + Default;

    /// Converting trait to take a source type and convert to `AccountId`.
    ///
    /// Used to define the type and conversion mechanism for referencing
    /// accounts in transactions. It's perfectly reasonable for this to be an
    /// identity conversion (with the source type being `AccountId`), but other
    /// modules (e.g. Indices module) may provide more functional/efficient
    /// alternatives.
    type Lookup: StaticLookup<Target = Self::AccountId>;

    /// The block header.
    type Header: Parameter
        + Header<Number = Self::BlockNumber, Hash = Self::Hash>
        + DeserializeOwned;

    /// The aggregated event type of the runtime.
    type Event: Parameter + Member + From<Event>;

    /// The `SignedExtension` to the basic transaction logic.
    type SignedExtra: SignedExtension;

    /// Creates the `SignedExtra` from the account nonce.
    fn extra(nonce: Self::Index) -> Self::SignedExtra;
}

/// The System extension trait for the Client.
pub trait SystemStore {
    /// System type.
    type System: System;

    /// Returns the account nonce for an account_id.
    fn account_nonce(
        &self,
        account_id: <Self::System as System>::AccountId,
    ) -> Box<dyn Future<Item = <Self::System as System>::Index, Error = Error> + Send>;
}

impl<T: System + 'static> SystemStore for Client<T> {
    type System = T;

    fn account_nonce(
        &self,
        account_id: <Self::System as System>::AccountId,
    ) -> Box<dyn Future<Item = <Self::System as System>::Index, Error = Error> + Send>
    {
        let account_nonce_map = || {
            Ok(self
                .metadata
                .module("System")?
                .storage("AccountNonce")?
                .get_map()?)
        };
        let map = match account_nonce_map() {
            Ok(map) => map,
            Err(err) => return Box::new(future::err(err)),
        };
        Box::new(self.fetch_or(map.key(account_id), map.default()))
    }
}

/// The System extension trait for the XtBuilder.
pub trait SystemCalls {
    /// System type.
    type System: System;

    /// Sets the new code.
    fn set_code(
        &mut self,
        code: Vec<u8>,
    ) -> Box<dyn Future<Item = <Self::System as System>::Hash, Error = Error> + Send>;
}

impl<T: System + 'static, P> SystemCalls for XtBuilder<T, P>
where
    P: Pair,
    P::Public: Into<<<T as System>::Lookup as StaticLookup>::Source>,
    P::Signature: Codec,
{
    type System = T;

    fn set_code(
        &mut self,
        code: Vec<u8>,
    ) -> Box<dyn Future<Item = <Self::System as System>::Hash, Error = Error> + Send>
    {
        let set_code_call =
            || Ok(self.metadata().module("System")?.call("set_code", code)?);
        let call = match set_code_call() {
            Ok(call) => call,
            Err(err) => return Box::new(future::err(err)),
        };
        Box::new(self.submit(call))
    }
}
