#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_std::vec::Vec;
use sp_runtime::traits::Hash;
use frame_support::traits::Currency;
use frame_support::dispatch::DispatchResult;
use frame_system::pallet_prelude::*;
use sp_runtime::print;

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub enum Issuance {
	Unlimited,
	Single,
	Limited,
	Stack,
}

impl Default for Issuance {
    fn default() -> Self { Issuance::Single }
}

#[derive(Encode, Decode, Clone, Default, PartialEq)]

pub struct SaleHistory<BlockNumber, AccountId, BalanceOf>{
	pub block: BlockNumber,
	pub seller: AccountId,
	pub buyer: AccountId,
	pub price: BalanceOf,
	pub copy: u16,
	pub quantity: u64
}

#[derive(Encode, Decode, Clone, Default, PartialEq)]
pub struct NFTv2<AccountId, Hash, BlockNumber, BalanceOf>{
	pub id: Hash,
	pub creator: AccountId,
	pub date: BlockNumber,
	pub royalty: u8,
	pub share: u8,
	pub data: Vec<u8>,
	pub issue: Issuance, 
	pub copy: u16,
	pub amount: u64,
	pub salt: u32,
	pub price: BalanceOf,
	pub target: BlockNumber,
	pub quantity: u64,
}



pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

use frame_support::{pallet_prelude::*, traits::ExistenceRequirement};
use sp_runtime::{SaturatedConversion, traits::Saturating};
use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
	}

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub type NFTv2_<T> = NFTv2<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash, <T as frame_system::Config>::BlockNumber, BalanceOf<T>>;
	pub type SaleHistory_<T> = SaleHistory<<T as frame_system::Config>::BlockNumber, <T as frame_system::Config>::AccountId, BalanceOf<T>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// =================================================================================
	// == Global

	#[pallet::storage]
	#[pallet::getter(fn get_record)]
	pub(super) type Database<T> = StorageDoubleMap<
		_, 
		Blake2_128Concat, <T as frame_system::Config>::Hash, 
		Blake2_128Concat, u16,
		<T as frame_system::Config>::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn get_count)]
	pub(super) type Counter<T> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_next)]
	pub(super) type NextIndex<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::Hash, u16, ValueQuery>;

	// =================================================================================

	#[pallet::storage]
	#[pallet::getter(fn get_lot)]
	pub(super) type Auction<T> = StorageDoubleMap<_, 
		Blake2_128Concat, <T as frame_system::Config>::AccountId, 
		Blake2_128Concat, (<T as frame_system::Config>::Hash, u16), 
		NFTv2_<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_expired)]
	pub(super) type GarbageCollector<T> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::BlockNumber, <T as frame_system::Config>::Hash>;

	// =================================================================================

	#[pallet::storage]
	#[pallet::getter(fn get_sale)]
	pub(super) type History<T> = StorageDoubleMap<_, 
		Blake2_128Concat, <T as frame_system::Config>::Hash, 
		Blake2_128Concat, <T as frame_system::Config>::Hash, 
		SaleHistory_<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_sale_count)]
	pub(super) type HistoryCounter<T> = StorageValue<_, u128, ValueQuery>;

	// =================================================================================
	// == Local

	#[pallet::storage]
	#[pallet::getter(fn get_nft)]
	pub(super) type NFTs<T> = StorageDoubleMap<_, 
		Blake2_128Concat, <T as frame_system::Config>::AccountId, 
		Blake2_128Concat, (<T as frame_system::Config>::Hash, u16), 
		NFTv2_<T>>;

	// =================================================================================
	// == Event

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		ExistError,
		GetError,
		InsufficientFunds,
		RoyaltyOver100,
		PriceMismatch,
		CopyLimitExceeded,
		AuctionExpired,
		U16Overflow,
		U128Overflow,
		QuantitySetTooHigh,
		QuantityMustBeOne,
		QuantityCannotBeZero,
		PurchaseQuantityCappedAt100,
		AvailableAmountLowerThanQuantity,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T:Config> Pallet<T> {
		
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn mint_single(origin: OriginFor<T>, data: Vec<u8>, royalty: u8) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			
			ensure!(royalty <= 100, Error::<T>::RoyaltyOver100);

			let mut nft= NFTv2 {
				id: T::Hash::default(),
				creator: caller.clone(),
				date: <frame_system::Pallet<T>>::block_number(),
				royalty,
				share: 100,
				data,
				issue: Issuance::Single,
				copy: 1,
				amount: 1,
				salt: 0,
				price: BalanceOf::<T>::default(),
				target: T::BlockNumber::default(),
				quantity: 0,
			};

			let mut id = T::Hashing::hash_of(&nft);

			while Database::<T>::get(id, 1) != None {						// collision detection
				nft.salt = nft.salt + 1;
				id = T::Hashing::hash_of(&nft);
			}

			nft.id = id;

			Database::<T>::insert(id,1, &caller);

			Counter::<T>::put(Counter::<T>::get() + 1);

			NFTs::<T>::insert(&caller, (id,1), &nft);

			//Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn mint_limited(origin: OriginFor<T>, data: Vec<u8>, copies: u16, royalty: u8) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			
			ensure!(royalty <= 100, Error::<T>::RoyaltyOver100);
			ensure!(copies <= 1000, Error::<T>::CopyLimitExceeded);

			let mut nft= NFTv2 {
				id: T::Hash::default(),
				creator: caller.clone(),
				date: <frame_system::Pallet<T>>::block_number(),
				royalty,
				share: 100,
				data,
				issue: Issuance::Limited,
				copy: 1,
				amount: 1,
				salt: 0,
				price: BalanceOf::<T>::default(),
				target: T::BlockNumber::default(),
				quantity: 0,
			};

			let mut id = T::Hashing::hash_of(&nft);

			while Database::<T>::get(id, 1) != None {						// collision detection
				nft.salt = nft.salt + 1;
				id = T::Hashing::hash_of(&nft);
			}

			nft.id = id;

			for copy in 1..=copies{
				nft.copy = copy;

				Database::<T>::insert(id,copy, &caller);
				NFTs::<T>::insert(&caller, (id,copy), &nft);
			}
			Counter::<T>::put(Counter::<T>::get() + copies as u128);

			//Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn mint_unlimited(origin: OriginFor<T>, data: Vec<u8>, royalty: u8) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			
			ensure!(royalty <= 100, Error::<T>::RoyaltyOver100);

			let mut nft= NFTv2 {
				id: T::Hash::default(),
				creator: caller.clone(),
				date: <frame_system::Pallet<T>>::block_number(),
				royalty,
				share: 100,
				data,
				issue: Issuance::Unlimited,
				copy: 0,
				amount: 1,
				salt: 0,
				price: BalanceOf::<T>::default(),
				target: T::BlockNumber::default(),
				quantity: 0,
			};

			let mut id = T::Hashing::hash_of(&nft);

			while Database::<T>::get(id, 1) != None {						// collision detection
				nft.salt = nft.salt + 1;
				id = T::Hashing::hash_of(&nft);
			}

			nft.id = id;

			Database::<T>::insert(id,1, &caller);
			Counter::<T>::put(Counter::<T>::get() + 1);
			NextIndex::<T>::insert(id, 2);

			NFTs::<T>::insert(&caller, (id,1), &nft);

			//Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn mint_stack(origin: OriginFor<T>, data: Vec<u8>, amount: u64, royalty: u8) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			
			ensure!(royalty <= 100, Error::<T>::RoyaltyOver100);

			let mut nft= NFTv2 {
				id: T::Hash::default(),
				creator: caller.clone(),
				date: <frame_system::Pallet<T>>::block_number(),
				royalty,
				share: 100,
				data,
				issue: Issuance::Stack,
				copy: 1,
				amount,
				salt: 0,
				price: BalanceOf::<T>::default(),
				target: T::BlockNumber::default(),
				quantity: 0,
			};

			let mut id = T::Hashing::hash_of(&nft);

			while Database::<T>::get(id, 1) != None {						// collision detection
				nft.salt = nft.salt + 1;
				id = T::Hashing::hash_of(&nft);
			}

			nft.id = id;

			Database::<T>::insert(id,1, &caller);
			Counter::<T>::put(Counter::<T>::get() + 1);
			NextIndex::<T>::insert(id, 2);

			NFTs::<T>::insert(&caller, (id,1), &nft);

			//Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn burn(origin: OriginFor<T>, id: T::Hash, index: u16) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			Auction::<T>::remove(&caller, (id, index));
			NFTs::<T>::remove(&caller, (id,index));
			Database::<T>::remove(id,index);

			// Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}
		
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn transfer(origin: OriginFor<T>, to: T::AccountId, id: T::Hash, index: u16) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let nft: NFTv2_<T> = NFTs::<T>::get(&caller, (id,index)).ok_or(Error::<T>::GetError)?;

			let lot: NFTv2_<T> = Auction::<T>::get(&caller, (id, index)).ok_or(Error::<T>::GetError)?;
			
			NFTs::<T>::remove(&caller, (id,index));
			NFTs::<T>::insert(&to, (id,index), nft);

			Auction::<T>::remove(&caller, (id, index));
			Auction::<T>::insert(&to, (id, index), lot);

			Database::<T>::insert(id,index, &to);

			// Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn sell(origin: OriginFor<T>, id: T::Hash, index: u16, price: BalanceOf<T>, sample: Vec<u8>, quantity: u64, day: u8) -> DispatchResult {
			let seller = ensure_signed(origin)?;

			let mut nft: NFTv2_<T> = NFTs::<T>::get(&seller, (id,index)).ok_or(Error::<T>::GetError)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			
			let block_delay: u32 = 28000 * day as u32;

			ensure!(nft != NFTv2::default(), Error::<T>::ExistError);
			ensure!(quantity != 0, Error::<T>::QuantityCannotBeZero);
			if nft.issue != Issuance::Unlimited { ensure!(nft.amount >= quantity, Error::<T>::QuantitySetTooHigh); }
			if nft.issue != Issuance::Stack { ensure!(quantity == 1, Error::<T>::QuantityMustBeOne); }

			nft.target = current_block_number + block_delay.into();
			nft.data = sample;
			nft.price = price;
			nft.quantity = quantity;

			Auction::<T>::remove(&seller, (id, index));
			Auction::<T>::insert(&seller, (id, index), nft);

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		fn buy(origin: OriginFor<T>, seller: T::AccountId, id: T::Hash, index: u16, price: BalanceOf<T>, quantity: u64) -> DispatchResult {
			let buyer = ensure_signed(origin.clone())?;

			let total_cost = price * quantity.saturated_into();	// IMPLEMENT SAFE MATH - checked_mul or saturating_mul

			ensure!(total_cost < T::Currency::total_balance(&buyer), Error::<T>::InsufficientFunds);

			let lot: NFTv2_<T> = Auction::<T>::get(&seller, (id, index)).ok_or(Error::<T>::GetError)?;

			ensure!(lot.price == price, Error::<T>::PriceMismatch);
			ensure!(lot.target > <frame_system::Pallet<T>>::block_number(), Error::<T>::AuctionExpired);
			
			if lot.issue != Issuance::Unlimited {
				ensure!(lot.quantity >= quantity, Error::<T>::QuantitySetTooHigh);
				ensure!(lot.amount >= quantity, Error::<T>::AvailableAmountLowerThanQuantity);
			}

			if lot.issue == Issuance::Single || lot.issue == Issuance::Limited {
				Pallet::<T>::transfer_single_or_limited(origin.clone(), seller.clone(), id, index.clone())?;				// transfer NFT
			}

			if lot.issue == Issuance::Unlimited {
				ensure!(!quantity > 100, Error::<T>::PurchaseQuantityCappedAt100);
				for _copy in 1..=quantity {
					Pallet::<T>::transfer_unlimited(origin.clone(), seller.clone(), id, index.clone(), 1 as u64)?;		// create a copy of NFT
				}
			}
			
			if lot.issue == Issuance::Stack {
				if NFTs::<T>::get(&buyer, (id,1)) != None {																// Check if buyer already owns a stack
					Pallet::<T>::increment_stack(origin.clone(), seller.clone(), id, 1 as u16, quantity.clone())?;			// increment amount to existing NFT
				} else {
					Pallet::<T>::transfer_stack(origin.clone(), seller.clone(), id, 1 as u16, quantity.clone())?;			// create a copy of NFT
				}
			}

			if lot.royalty == 0 {
				T::Currency::transfer(&buyer, &seller, total_cost, ExistenceRequirement::AllowDeath)?;
			} else if lot.creator == seller {
				T::Currency::transfer(&buyer, &seller, total_cost, ExistenceRequirement::AllowDeath)?;
			} else {
				let royalty: BalanceOf<T> = lot.royalty.saturated_into();
				
				let h: u8 = 100.into();
				let hundred: BalanceOf<T> = h.into();
				
				let s: u32 = 100.saturating_sub(lot.royalty.into());
				let sellers_proportion: BalanceOf<T> = s.saturated_into();

				let royalty_fee = (total_cost * royalty) / hundred;
				let seller_fee = (total_cost * sellers_proportion) / hundred;
				
				T::Currency::transfer(&buyer, &seller, seller_fee, ExistenceRequirement::KeepAlive)?;
				T::Currency::transfer(&buyer, &lot.creator, royalty_fee, ExistenceRequirement::KeepAlive)?;

				// Mathematics explained
				// 		Amount payable to creator: (price * nft.royalty) / 100;
				// 		Amount payable to seller:  (price * (100 - nft.royalty)) / 100;
			}

		 	// Self::deposit_event(Event::SomethingStored(something, caller));

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {

	fn transfer_single_or_limited(origin: OriginFor<T>, from: T::AccountId, id: T::Hash, index: u16) -> DispatchResult {
		let to = ensure_signed(origin)?;

		let nft: NFTv2_<T> = NFTs::<T>::get(&from, (id,index)).ok_or(Error::<T>::GetError)?;

		let lot: NFTv2_<T> = Auction::<T>::get(&from, (id, index)).ok_or(Error::<T>::GetError)?;

		let sale_history: SaleHistory_<T> = SaleHistory {
			block: <frame_system::Pallet<T>>::block_number(),
			seller: from.clone(),
			buyer: to.clone(),
			price: lot.price,
			copy: 1,
			quantity: 1,
		};

		let purchase_id = T::Hashing::hash_of(&sale_history);

		Auction::<T>::remove(&from, (id, index));

		NFTs::<T>::remove(&from, (id,index));
		NFTs::<T>::insert(&to, (id,index), nft);

		Database::<T>::insert(id,index, &to);

		History::<T>::insert(id, purchase_id, sale_history);
				

		HistoryCounter::<T>::put(HistoryCounter::<T>::get().checked_add(1).ok_or(Error::<T>::U128Overflow)?);

		// Self::deposit_event(Event::SomethingStored(something, caller));

		Ok(())
	}

	fn transfer_unlimited(origin: OriginFor<T>, seller: T::AccountId, id: T::Hash, index: u16, quantity:u64) -> DispatchResult {
		let to = ensure_signed(origin)?;

		let mut nft: NFTv2_<T> = NFTs::<T>::get(&seller, (id,index)).ok_or(Error::<T>::GetError)?;

		let lot: NFTv2_<T> = Auction::<T>::get(&seller, (id, index)).ok_or(Error::<T>::GetError)?;

		nft.issue = Issuance::Single;

		let next_index: u16 = NextIndex::<T>::get(id);

		let sale_history: SaleHistory_<T> = SaleHistory {
			block: <frame_system::Pallet<T>>::block_number(),
			seller: seller.clone(),
			buyer: to.clone(),
			price: lot.price,
			copy: next_index,
			quantity,
		};

		let purchase_id = T::Hashing::hash_of(&sale_history);

		NFTs::<T>::insert(&to, (id,next_index), nft);

		Database::<T>::insert(id,next_index, &to);

		NextIndex::<T>::insert(id, next_index.checked_add(1).ok_or(Error::<T>::U16Overflow)?);

		History::<T>::insert(id, purchase_id, sale_history);		

		HistoryCounter::<T>::put(HistoryCounter::<T>::get().checked_add(1).ok_or(Error::<T>::U128Overflow)?);

		// Self::deposit_event(Event::SomethingStored(something, caller));

		Ok(())
	}

	fn transfer_stack(origin: OriginFor<T>, seller: T::AccountId, id: T::Hash, index: u16, quantity:u64) -> DispatchResult {
		let buyer = ensure_signed(origin)?;

		let mut seller_nft: NFTv2_<T> = NFTs::<T>::get(&seller, (id,index)).ok_or(Error::<T>::GetError)?;

		let mut nft: NFTv2_<T> = seller_nft.clone();

		let mut lot: NFTv2_<T> = Auction::<T>::get(&seller, (id, index)).ok_or(Error::<T>::GetError)?;

		let next_index = NextIndex::<T>::get(id);

		let sale_history: SaleHistory_<T> = SaleHistory {
			block: <frame_system::Pallet<T>>::block_number(),
			seller: seller.clone(),
			buyer: buyer.clone(),
			price: lot.price,
			copy: next_index,
			quantity,
		};

		let purchase_id = T::Hashing::hash_of(&sale_history);
		
		nft.amount = quantity;
		nft.copy = next_index;
		lot.amount = lot.amount - quantity;
		lot.quantity = lot.quantity - quantity;
		seller_nft.amount = seller_nft.amount - quantity;

		NFTs::<T>::insert(&buyer, (id,index), nft);

		Database::<T>::insert(id,next_index, &buyer);

		NextIndex::<T>::insert(id, next_index.checked_add(1).ok_or(Error::<T>::U16Overflow)?);

		History::<T>::insert(id, purchase_id, sale_history);	

		HistoryCounter::<T>::put(HistoryCounter::<T>::get().checked_add(1).ok_or(Error::<T>::U128Overflow)?);
		
		if lot.quantity == 0 {														// if stack reduced to 0, remove from Auction.
			Auction::<T>::remove(&seller, (id, index));
		} else {
			Auction::<T>::insert(&seller, (id, index), lot);
		}

		if seller_nft.amount == 0 && seller.clone() != seller_nft.creator {
			NFTs::<T>::remove(&seller, (id,index));
			Database::<T>::remove(id,index);
		} else {
			NFTs::<T>::insert(&seller, (id,index), seller_nft);
		}

		// Self::deposit_event(Event::SomethingStored(something, caller));

		Ok(())
	}

	fn increment_stack(origin: OriginFor<T>, seller: T::AccountId, id: T::Hash, index: u16, quantity:u64) -> DispatchResult {
		let buyer = ensure_signed(origin)?;

		let mut nft: NFTv2_<T> = NFTs::<T>::get(&buyer, (id,index)).ok_or(Error::<T>::GetError)?;

		let mut seller_nft: NFTv2_<T> = NFTs::<T>::get(&seller, (id,index)).ok_or(Error::<T>::GetError)?;

		let mut lot: NFTv2_<T> = Auction::<T>::get(&seller, (id, index)).ok_or(Error::<T>::GetError)?;
		
		let existing_index = nft.copy;

		let sale_history: SaleHistory_<T> = SaleHistory {
			block: <frame_system::Pallet<T>>::block_number(),
			seller: seller.clone(),
			buyer: buyer.clone(),
			price: lot.price,
			copy: existing_index,
			quantity,
		};

		let purchase_id = T::Hashing::hash_of(&sale_history);
		
		nft.amount = nft.amount + quantity;
		lot.amount = lot.amount - quantity;
		lot.quantity = lot.quantity - quantity;
		seller_nft.amount = seller_nft.amount - quantity;

		NFTs::<T>::insert(&buyer, (id,index), nft);

		NFTs::<T>::insert(&seller, (id,index), &seller_nft);

		History::<T>::insert(id, purchase_id, sale_history);		

		HistoryCounter::<T>::put(HistoryCounter::<T>::get().checked_add(1).ok_or(Error::<T>::U128Overflow)?);
		
		if lot.quantity == 0 {														// if stack reduced to 0, remove from Auction.
			Auction::<T>::remove(&seller, (id, index));
		} else {
			Auction::<T>::insert(&seller, (id, index), lot);
		}

		if seller_nft.amount == 0 && seller.clone() != seller_nft.creator {
			NFTs::<T>::remove(&seller, (id,index));
			Database::<T>::remove(id,existing_index);
		} else {
			NFTs::<T>::insert(&seller, (id,index), seller_nft);
		}

		// Self::deposit_event(Event::SomethingStored(something, caller));

		Ok(())
	}
}
