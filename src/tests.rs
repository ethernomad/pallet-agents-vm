use crate::*;
use frame_support::{assert_err, assert_ok, derive_impl, parameter_types};
use sp_core::H256;
use sp_runtime::{
    traits::{BadOrigin, BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchError,
};
// Reexport crate as its pallet name for construct_runtime.
use crate as pallet_agents_vm;

type Block = frame_system::mocking::MockBlock<Test>;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        AgentsVM: pallet_agents_vm,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Hash = H256;
    type RuntimeCall = RuntimeCall;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type AccountStore = System;
}

parameter_types! {
    pub const BridgePalletId: PalletId = PalletId(*b"evmbridg");
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;

    type PalletId = BridgePalletId;
    type Currency = Balances;
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = RuntimeGenesisConfig {
        // We use default for brevity, but you can configure as desired if needed.
        system: Default::default(),
        balances: Default::default(),
    }
    .build_storage()
    .unwrap();
    let genesis = pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 100), (2, 100)],
    };
    genesis.assimilate_storage(&mut t).unwrap();
    t.into()
}

#[test]
fn set_deposit_amount() {
    new_test_ext().execute_with(|| {
        let deposit_amount = 133;
        assert_err!(
            AgentsVM::set_deposit_amount(RuntimeOrigin::signed(1), deposit_amount),
            BadOrigin
        );
        assert_ok!(AgentsVM::set_deposit_amount(
            RuntimeOrigin::root(),
            deposit_amount
        ));
        assert_eq!(DepositAmount::<Test>::get(), deposit_amount);
        let deposit_amount = 134;
        assert_ok!(AgentsVM::set_deposit_amount(
            RuntimeOrigin::root(),
            deposit_amount
        ));
        assert_eq!(DepositAmount::<Test>::get(), deposit_amount);
    });
}

#[test]
fn set_slash_amount() {
    new_test_ext().execute_with(|| {
        let slash_amount = 133;
        assert_err!(
            AgentsVM::set_slash_amount(RuntimeOrigin::signed(1), slash_amount),
            BadOrigin
        );
        assert_ok!(AgentsVM::set_slash_amount(
            RuntimeOrigin::root(),
            slash_amount
        ));
        assert_eq!(SlashAmount::<Test>::get(), slash_amount);
        let slash_amount = 134;
        assert_ok!(AgentsVM::set_slash_amount(
            RuntimeOrigin::root(),
            slash_amount
        ));
        assert_eq!(SlashAmount::<Test>::get(), slash_amount);
    });
}

#[test]
fn register() {
    new_test_ext().execute_with(|| {
        let deposit_amount = 10;
        assert_ok!(AgentsVM::set_deposit_amount(
            RuntimeOrigin::root(),
            deposit_amount
        ));
        let name = "Node1".to_string();
        let uri = "https://node.network:443".to_string();
        assert_err!(
            AgentsVM::register(RuntimeOrigin::signed(3), name.to_string(), uri.to_string()),
            DispatchError::Other("Can't transfer value.")
        );
        assert_ok!(AgentsVM::register(
            RuntimeOrigin::signed(1),
            name.clone(),
            uri.clone(),
        ));
        let accounts = Accounts::<Test>::get();
        assert_eq!(accounts.contains(&1), true);
        assert_eq!(Deposit::<Test>::get(1), deposit_amount);
        assert_err!(
            AgentsVM::register(RuntimeOrigin::signed(1), name.to_string(), uri.to_string(),),
            Error::<Test>::NodeAlreadyRegistered
        );
        let node = AccountNode::<Test>::get(1);
        assert_eq!(node, Some(Node { name, uri }));
    });
}

#[test]
fn deregister() {
    new_test_ext().execute_with(|| {
        let deposit_amount = 10;
        assert_ok!(AgentsVM::set_deposit_amount(
            RuntimeOrigin::root(),
            deposit_amount
        ));
        assert_err!(
            AgentsVM::deregister(RuntimeOrigin::signed(1),),
            Error::<Test>::NodeNotRegistered
        );
        let name = "Node1";
        let uri = "https://node.network:443";
        assert_ok!(AgentsVM::register(
            RuntimeOrigin::signed(1),
            name.to_string(),
            uri.to_string(),
        ));
        assert_ok!(AgentsVM::deregister(RuntimeOrigin::signed(1),));
        let accounts = Accounts::<Test>::get();
        assert_eq!(accounts.contains(&1), false);
        assert_eq!(Deposit::<Test>::get(1), 0);
        assert_eq!(AccountNode::<Test>::get(1), None);
    });
}

#[test]
fn reward() {
    new_test_ext().execute_with(|| {
        let reward_amount = 8;
        assert_err!(
            AgentsVM::reward(RuntimeOrigin::signed(1), 1, reward_amount),
            BadOrigin
        );
        assert_err!(
            AgentsVM::reward(RuntimeOrigin::root(), 1, reward_amount),
            Error::<Test>::NodeNotRegistered
        );
        assert_ok!(AgentsVM::set_deposit_amount(RuntimeOrigin::root(), 10));
        let name = "Node1";
        let uri = "https://node.network:443";
        assert_ok!(AgentsVM::register(
            RuntimeOrigin::signed(1),
            name.to_string(),
            uri.to_string(),
        ));
        assert_eq!(Deposit::<Test>::get(1), 10);
        assert_ok!(AgentsVM::reward(RuntimeOrigin::root(), 1, reward_amount));
        assert_eq!(Deposit::<Test>::get(1), 18);
    });
}

#[test]
fn slash() {
    new_test_ext().execute_with(|| {
        let slash_amount = 8;
        assert_err!(
            AgentsVM::slash(RuntimeOrigin::signed(1), 1, slash_amount),
            BadOrigin
        );
        assert_err!(
            AgentsVM::slash(RuntimeOrigin::root(), 1, slash_amount),
            Error::<Test>::NodeNotRegistered
        );
        assert_ok!(AgentsVM::set_deposit_amount(RuntimeOrigin::root(), 10));
        let name = "Node1";
        let uri = "https://node.network:443";
        assert_ok!(AgentsVM::register(
            RuntimeOrigin::signed(1),
            name.to_string(),
            uri.to_string(),
        ));
        assert_eq!(Deposit::<Test>::get(1), 10);
        assert_ok!(AgentsVM::slash(RuntimeOrigin::root(), 1, slash_amount));
        assert_eq!(Deposit::<Test>::get(1), 2);
    });
}
