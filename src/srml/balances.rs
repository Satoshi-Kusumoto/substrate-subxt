//! Implements support for the srml_balances module.
use crate::{
    codec::compact,
    error::Error,
    srml::system::System,
    Client,
    XtBuilder,
};
use futures::future::{
    self,
    Future,
};
use parity_scale_codec::Codec;
use runtime_primitives::traits::{
    MaybeSerializeDebug,
    Member,
    SimpleArithmetic,
    StaticLookup,
};
use runtime_support::Parameter;
use substrate_primitives::Pair;

/// The subset of the `srml_balances::Trait` that a client must implement.
pub trait Balances: System {
    /// The balance of an account.
    type Balance: Parameter
        + Member
        + SimpleArithmetic
        + Codec
        + Default
        + Copy
        + MaybeSerializeDebug
        + From<<Self as System>::BlockNumber>;
}

/// The Balances extension trait for the Client.
pub trait BalancesStore {
    /// Balances type.
    type Balances: Balances;

    /// The 'free' balance of a given account.
    ///
    /// This is the only balance that matters in terms of most operations on
    /// tokens. It alone is used to determine the balance when in the contract
    ///  execution environment. When this balance falls below the value of
    ///  `ExistentialDeposit`, then the 'current account' is deleted:
    ///  specifically `FreeBalance`. Further, the `OnFreeBalanceZero` callback
    /// is invoked, giving a chance to external modules to clean up data
    /// associated with the deleted account.
    ///
    /// `system::AccountNonce` is also deleted if `ReservedBalance` is also
    /// zero. It also gets collapsed to zero if it ever becomes less than
    /// `ExistentialDeposit`.
    fn free_balance(
        &self,
        account_id: <Self::Balances as System>::AccountId,
    ) -> Box<dyn Future<Item = <Self::Balances as Balances>::Balance, Error = Error> + Send>;
}

impl<T: Balances + 'static> BalancesStore for Client<T> {
    type Balances = T;

    fn free_balance(
        &self,
        account_id: <Self::Balances as System>::AccountId,
    ) -> Box<dyn Future<Item = <Self::Balances as Balances>::Balance, Error = Error> + Send>
    {
        let free_balance_map = || {
            Ok(self
                .metadata()
                .module("Balances")?
                .storage("FreeBalance")?
                .get_map::<
                <Self::Balances as System>::AccountId,
                <Self::Balances as Balances>::Balance>()?)
        };
        let map = match free_balance_map() {
            Ok(map) => map,
            Err(err) => return Box::new(future::err(err)),
        };
        Box::new(self.fetch_or(map.key(account_id), map.default()))
    }
}

/// The Balances extension trait for the XtBuilder.
pub trait BalancesCalls {
    /// Balances type.
    type Balances: Balances;

    /// Transfer some liquid free balance to another account.
    ///
    /// `transfer` will set the `FreeBalance` of the sender and receiver.
    /// It will decrease the total issuance of the system by the `TransferFee`.
    /// If the sender's account is below the existential deposit as a result
    /// of the transfer, the account will be reaped.
    fn transfer(
        &mut self,
        to: <<Self::Balances as System>::Lookup as StaticLookup>::Source,
        amount: <Self::Balances as Balances>::Balance,
    ) -> Box<dyn Future<Item = <Self::Balances as System>::Hash, Error = Error> + Send>;
}

impl<T: Balances + 'static, P> BalancesCalls for XtBuilder<T, P>
where
    P: Pair,
    P::Public: Into<<<T as System>::Lookup as StaticLookup>::Source>,
    P::Signature: Codec,
{
    type Balances = T;

    fn transfer(
        &mut self,
        to: <<Self::Balances as System>::Lookup as StaticLookup>::Source,
        amount: <Self::Balances as Balances>::Balance,
    ) -> Box<dyn Future<Item = <Self::Balances as System>::Hash, Error = Error> + Send>
    {
        let transfer_call = || {
            Ok(self
                .metadata()
                .module("Balances")?
                .call("transfer", (to, compact(amount)))?)
        };
        let call = match transfer_call() {
            Ok(call) => call,
            Err(err) => return Box::new(future::err(err)),
        };
        Box::new(self.submit(call))
    }
}
