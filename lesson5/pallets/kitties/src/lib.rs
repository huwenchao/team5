#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{decl_module, decl_storage, decl_error, ensure, StorageValue, StorageMap, traits::Randomness};
use sp_io::hashing::blake2_128;
use frame_system::ensure_signed;
use sp_runtime::{DispatchError, DispatchResult};
use sp_runtime::traits::StaticLookup;


#[derive(Encode, Decode)]
pub struct Kitty(pub [u8; 16]);

pub trait Trait: frame_system::Trait {
}

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id / index
		pub Kitties get(fn kitties): map hasher(blake2_128_concat) u32 => Option<Kitty>;
		/// Stores the total number of kitties. i.e. the next kitty index
		pub KittiesCount get(fn kitties_count): u32;

		/// Get kitty ID by account ID and user kitty index
		pub OwnedKitties get(fn owned_kitties): map hasher(blake2_128_concat) (T::AccountId, u32) => u32;
		/// Get number of kitties by account ID
		pub OwnedKittiesCount get(fn owned_kitties_count): map hasher(blake2_128_concat) T::AccountId => u32;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		KittiesCountOverflow,
		InvalidKittyId,
		RequireDifferentParent,
		UserNotHaveTheKitty,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		/// Create a new kitty
		#[weight = 0]
		pub fn create(origin) {
			let sender = ensure_signed(origin)?;
			let kitty_id = Self::next_kitty_id()?;

			// Generate a random 128bit value
			let dna = Self::random_value(&sender);

			// Create and store kitty
			let kitty = Kitty(dna);

			// 作业：补完剩下的部分
			Self::insert_kitty(sender, kitty_id, kitty);
		}

		/// Breed kitties
		#[weight = 0]
		pub fn breed(origin, kitty_id_1: u32, kitty_id_2: u32) {
			let sender = ensure_signed(origin)?;

			Self::do_breed(sender, kitty_id_1, kitty_id_2)?;
		}

		#[weight = 0]
		pub fn transfer(origin, user_kitty_id: u32, to: <T::Lookup as StaticLookup>::Source) {
			let sender = ensure_signed(origin)?;

			let from_user_kitties_count = OwnedKittiesCount::<T>::get(&sender);
			ensure!(from_user_kitties_count > user_kitty_id, Error::<T>::UserNotHaveTheKitty);

			// check to user kitties will not flow
			let to = T::Lookup::lookup(to)?;
			let to_user_kitties_count = OwnedKittiesCount::<T>::get(&to);
			if to_user_kitties_count == u32::max_value() {
				return Err(Error::<T>::KittiesCountOverflow.into());
			}

			// remove the from user kitty
			let kitty_id = OwnedKitties::<T>::get((&sender, user_kitty_id));
			OwnedKittiesCount::<T>::insert(&sender, from_user_kitties_count - 1);
			if user_kitty_id + 1 != from_user_kitties_count {
				// move the last user kitty to the removed position
				let from_last_kitty_id = OwnedKitties::<T>::get((&sender, from_user_kitties_count - 1));
				OwnedKitties::<T>::remove((&sender, from_user_kitties_count - 1));
				OwnedKitties::<T>::insert((&sender, user_kitty_id), from_last_kitty_id);
			} else {
				OwnedKitties::<T>::remove((&sender, user_kitty_id));
			}

			// add the to user kitty
			OwnedKittiesCount::<T>::insert(&to, to_user_kitties_count + 1);
			OwnedKitties::<T>::insert((&to, to_user_kitties_count + 1), kitty_id);
		}
	}
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
	(selector & dna1) | (!selector & dna2)
}

impl<T: Trait> Module<T> {
	fn random_value(sender: &T::AccountId) -> [u8; 16] {
		// 作业：完成方法
		let payload = (
			<pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
			sender,
			<frame_system::Module<T>>::extrinsic_index(),
		);
		payload.using_encoded(blake2_128)
	}

	fn next_kitty_id() -> sp_std::result::Result<u32, DispatchError> {
		let kitty_id = Self::kitties_count();
		if kitty_id == u32::max_value() {
			return Err(Error::<T>::KittiesCountOverflow.into());
		}
		Ok(kitty_id)
	}

	fn insert_kitty(owner: T::AccountId, kitty_id: u32, kitty: Kitty) {
		// 作业：完成方法
		Kitties::insert(kitty_id, kitty);
		KittiesCount::put(kitty_id + 1);
		let user_kitties_count = OwnedKittiesCount::<T>::get(&owner);
		OwnedKittiesCount::<T>::insert(&owner, user_kitties_count + 1);
		OwnedKitties::<T>::insert((&owner, user_kitties_count), kitty_id);
	}

	fn do_breed(sender: T::AccountId, kitty_id_1: u32, kitty_id_2: u32) -> DispatchResult {
		let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
		let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

		ensure!(kitty_id_1 != kitty_id_2, Error::<T>::RequireDifferentParent);

		let kitty_id = Self::next_kitty_id()?;

		let kitty1_dna = kitty1.0;
		let kitty2_dna = kitty2.0;

		// Generate a random 128bit value
		let selector = Self::random_value(&sender);
		let mut new_dna = [0u8; 16];

		// Combine parents and selector to create new kitty
		for i in 0..kitty1_dna.len() {
			new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
		}

		Self::insert_kitty(sender, kitty_id, Kitty(new_dna));

		Ok(())
	}
}
