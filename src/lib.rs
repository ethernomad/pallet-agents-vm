// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use alloc::vec;
use codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use frame_system::ensure_signed;

use frame_support::{
    traits::{Currency, ExistenceRequirement::AllowDeath, Get},
    PalletId,
};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::traits::AccountIdConversion;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(feature = "try-runtime")]
// use sp_runtime::TryRuntimeError;
pub mod weights;
// pub use weights::*;

extern crate alloc;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Debug, PartialEq, Encode, Decode, TypeInfo)]
pub struct Node {
    name: String,
    uri: String,
}

/// Enable `dev_mode` for this pallet.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: pallet_balances::Config + frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet
        // type WeightInfo: WeightInfo;

        /// This is a normal Rust type, nothing specific to FRAME here.
        type Currency: Currency<Self::AccountId>;

        /// PalletId for pallet. An appropriate value could be ```PalletId(*b"evmbridg")```
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    // Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
    // method.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        pub fn set_deposit_amount(
            origin: OriginFor<T>,
            deposit_amount: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            <DepositAmount<T>>::set(deposit_amount);
            Self::deposit_event(Event::DepositAmountSet { deposit_amount });
            Ok(())
        }

        #[pallet::call_index(1)]
        pub fn set_slash_amount(
            origin: OriginFor<T>,
            slash_amount: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            <SlashAmount<T>>::set(slash_amount);
            Self::deposit_event(Event::SlashAmountSet { slash_amount });
            Ok(())
        }

        #[pallet::call_index(2)]
        pub fn register(origin: OriginFor<T>, name: String, uri: String) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let mut accounts = Accounts::<T>::get();
            if accounts.contains(&sender) {
                Err(Error::<T>::NodeAlreadyRegistered)?;
            }

            let deposit_amount = <DepositAmount<T>>::get();
            // Move the value from the sender to the pallet.
            T::Currency::transfer(
                &sender,
                &Self::pallet_account_id(),
                deposit_amount,
                AllowDeath,
            )
            .map_err(|_| DispatchError::Other("Can't transfer value."))?;
            // Put the new value into storage.
            <Deposit<T>>::insert(&sender, deposit_amount);
            let node = Node {
                name: name.clone(),
                uri: uri.clone(),
            };
            <AccountNode<T>>::insert(&sender, node);
            accounts.push(sender.clone());
            Accounts::<T>::set(accounts);
            // Let's deposit an event to let the outside world know this happened.
            Self::deposit_event(Event::NodeRegistered {
                account: sender,
                name,
                uri,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        pub fn deregister(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let mut accounts = Accounts::<T>::get();
            let i = accounts
                .iter()
                .position(|a| a == &sender)
                .ok_or(Error::<T>::NodeNotRegistered)?;
            accounts.swap_remove(i);
            Accounts::<T>::set(accounts);
            let deposit = <Deposit<T>>::get(&sender);
            <Deposit<T>>::remove(&sender);
            <AccountNode<T>>::remove(&sender);
            // Move the value from the pallet to the sender.
            T::Currency::transfer(&Self::pallet_account_id(), &sender, deposit, AllowDeath)
                .map_err(|_| DispatchError::Other("Can't transfer value."))?;
            Ok(())
        }

        #[pallet::call_index(4)]
        pub fn reward(
            origin: OriginFor<T>,
            account: T::AccountId,
            value: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let accounts = Accounts::<T>::get();
            if !accounts.contains(&account) {
                Err(Error::<T>::NodeNotRegistered)?;
            }
            let deposit = <Deposit<T>>::get(&account);
            <Deposit<T>>::set(&account, deposit + value);
            Self::deposit_event(Event::NodeRewarded { account, value });
            Ok(())
        }

        #[pallet::call_index(5)]
        pub fn slash(
            origin: OriginFor<T>,
            account: T::AccountId,
            mut value: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let mut deposit = <Deposit<T>>::get(&account);
            let accounts = Accounts::<T>::get();
            if !accounts.contains(&account) {
                Err(Error::<T>::NodeNotRegistered)?;
            }
            if deposit >= value {
                deposit -= value;
            } else {
                value = deposit;
                deposit -= deposit;
            }
            <Deposit<T>>::set(&account, deposit);
            Self::deposit_event(Event::NodeSlashed { account, value });
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DepositAmountSet {
            deposit_amount: BalanceOf<T>,
        },
        SlashAmountSet {
            slash_amount: BalanceOf<T>,
        },
        NodeRegistered {
            account: T::AccountId,
            name: String,
            uri: String,
        },
        NodeRewarded {
            account: T::AccountId,
            value: BalanceOf<T>,
        },
        NodeSlashed {
            account: T::AccountId,
            value: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NodeAlreadyRegistered,
        NodeNotRegistered,
    }

    #[pallet::storage]
    #[pallet::getter(fn deposit_amount)]
    pub type DepositAmount<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn slash_amount)]
    pub type SlashAmount<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn deposits)]
    pub type Deposit<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn nodes)]
    pub type AccountNode<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Node>;

    #[pallet::storage]
    #[pallet::getter(fn accounts)]
    pub type Accounts<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;
}

impl<T: Config> Pallet<T> {
    /// The account ID of the fund pot.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn pallet_account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }
}
