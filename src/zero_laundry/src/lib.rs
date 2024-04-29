// Import necessary crates and modules
#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{cell::RefCell, collections::HashMap};

// Define types for memory management
type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Define User struct
#[derive(candid::CandidType, Serialize, Deserialize, Default, Clone)]
struct User {
    id: u64,
    name: String,
    balance: u64,
    pending_orders: Vec<u64>,
    active_orders: Vec<u64>,
    completed_orders: Vec<u64>,
}

// Implement Storable trait for User
impl Storable for User {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Implement BoundedStorable trait for User
impl BoundedStorable for User {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

#[derive(candid::CandidType, Serialize, Deserialize, Default, Clone)]
struct Laundry {
    id: u64,
    weight: u64,
    package: String,
    amount_to_pay: u64,
    status: String,
    user_id: u64,
    created_at: u64,
    updated_at: Option<u64>,
    finished_at: Option<u64>,
}

// Implement Storable trait for Laundry
impl Storable for Laundry {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Implement BoundedStorable trait for Laundry
impl BoundedStorable for Laundry {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Define thread-local storage for memory management and counters
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
        .expect("cannot create a counter")
    );

    static USER_STORAGE: RefCell<StableBTreeMap<u64, User, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ));

    static LAUNDRY_STORAGE: RefCell<StableBTreeMap<u64, Laundry, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
        ));
}

// Payload for creating users
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct UserPayload {
    name: String,
}

// Payload for creating laundries
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct LaundryPayload {
    weight: u64,
    user_id: u64,
    package: String,
}

// Payload for paying for laundry
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct PayPayload {
    user_id: u64,
    laundry_id: u64,
}

// Enum for error handling
#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    InvalidInput { msg: String },
    InsufficientBalance { msg: String },
    AlreadyPaid { msg: String },
    LaundryNotDone { msg: String, remaining_time: (u64, u64) }, // Include remaining time
    LaundryAlreadyDone { msg: String },
}

// Function to add a user
#[ic_cdk::update]
fn add_user(payload: UserPayload) -> Option<User> {
    // Increment user ID counter and return the new value
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1)
    })
    .expect("cannot increment id counter");

    // Create new user
    let user = User {
        id,
        name: payload.name,
        balance: 100000,
        pending_orders: vec![],
        active_orders: vec![],
        completed_orders: vec![],
    };

    // Insert user into storage
    do_insert_user(&user);
    Some(user)
}

// Function to insert a user into storage
fn do_insert_user(user: &User) {
    USER_STORAGE.with(|service| service.borrow_mut().insert(user.id, user.clone()));
}

// Function to retrieve all users
#[ic_cdk::query]
fn get_all_users() -> Option<Vec<User>> {
    // Retrieve all users from storage
    let users_map: HashMap<u64, User> = USER_STORAGE.with(|service| service.borrow().iter().map(|(k, v)| (*k, v.clone())).collect());

    // Convert hashmap values to vector
    let users: Vec<User> = users_map.into_iter().map(|(_, user)| user).collect();

    // Return users if found
    if !users.is_empty() {
        Some(users)
    } else {
        None
    }
}

// Function to retrieve a user by ID
#[ic_cdk::query]
fn get_user_by_id(id: u64) -> Result<User, Error> {
    // Retrieve user by ID
    match get_user(&id) {
        Some(user) => Ok(user),
        None => Err(Error::NotFound { msg: "User not found".to_string() })
    }
}

// Function to get a user by ID
fn get_user(id: &u64) -> Option<User> {
    USER_STORAGE.with(|service| service.borrow().get(id))
}

// Function to add a laundry
#[ic_cdk::update]
fn add_laundry(payload: LaundryPayload) -> Result<Laundry, Error> {
    // Increment laundry ID counter and return the new value
    let id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1)
    })
    .expect("cannot increment id counter");

    // Calculate amount to pay based on package type
    let amount_to_pay: u64 = match payload.package.as_str() {
        "regular" => payload.weight * 6,
        "express" => payload.weight * 10,
        _ => return Err(Error::InvalidInput { msg: "Invalid package type".to_string()});

    // Create new laundry
    let laundry = Laundry {
        id,
        weight: payload.weight,
        user_id: payload.user_id,
        package: payload.package.clone(),
        status: "waiting for payment".to_string(),
        amount_to_pay,
        created_at: ic_cdk::api::time(),
        updated_at: None,
        finished_at: None,
    };

    // Insert laundry into storage
    do_insert_laundry(&laundry);

    // Update user's pending orders or create user if not found
    match get_user(&payload.user_id) {
        Some(mut user) => {
            user.pending_orders.push(laundry.id);
            do_insert_user(&user);
            Ok(laundry)
        }
        None => {
            // Create new user
            let new_user = User {
                id: payload.user_id,
                name: format!("User {}", payload.user_id),
                balance: 0,
                pending_orders: vec![laundry.id],
                active_orders: vec![],
                completed_orders: vec![],
            };
            do_insert_user(&new_user);
            Ok(laundry)
        }
    }
}

// Function to insert a laundry into storage
fn do_insert_laundry(laundry: &Laundry) {
    LAUNDRY_STORAGE.with(|service| service.borrow_mut().insert(laundry.id, laundry.clone()));
}

// Function to retrieve all laundries
#[ic_cdk::query]
fn get_all_laundries() -> Option<Vec<Laundry>> {
    // Retrieve all laundries from storage
    let laundries_map: HashMap<u64, Laundry> = LAUNDRY_STORAGE.with(|service| service.borrow().iter().map(|(k, v)| (*k, v.clone())).collect());

    // Convert hashmap values to vector
    let laundries: Vec<Laundry> = laundries_map.into_iter().map(|(_, laundry)| laundry).collect();

    // Return laundries if found
    if !laundries.is_empty() {
        Some(laundries)
    } else {
        None
    }
}

// Function to retrieve a laundry by ID
#[ic_cdk::query]
fn get_laundry_by_id(id: u64) -> Result<Laundry, Error> {
    // Retrieve laundry by ID
    match get_laundry(&id) {
        Some(laundry) => Ok(laundry),
        None => Err(Error::NotFound { msg: "Laundry not found".to_string() })
    }
}

// Function to get a laundry by ID
fn get_laundry(id: &u64) -> Option<Laundry> {
    LAUNDRY_STORAGE.with(|service| service.borrow().get(id))
}

// Function to pay for a laundry
#[ic_cdk::update]
fn pay_laundry(payload: PayPayload) -> Result<Laundry, Error> {
    // Retrieve user by ID
    match USER_STORAGE.with(|service| service.borrow().get(&payload.user_id)) {
        Some(mut user) => {
            // Retrieve laundry by ID
            let laundry = match get_laundry(&payload.laundry_id) {
                Some(laundry) => laundry,
                None => return Err(Error::NotFound { msg: "Laundry not found".to_string() }),
            };

            // Check if user has sufficient balance
            if user.balance < laundry.amount_to_pay {
                return Err(Error::InsufficientBalance { msg: "Insufficient balance".to_string() });
            }

            // Check if user ID matches laundry's user ID
            if payload.user_id != laundry.user_id {
                return Err(Error::InvalidInput { msg: "Invalid user".to_string() });
            }

            // Update user's balance and orders
            user.balance -= laundry.amount_to_pay;
            user.pending_orders.retain(|&x| x != payload.laundry_id);
            user.active_orders.push(payload.laundry_id);
            do_insert_user(&user);

            // Update laundry status and timestamps
            match LAUNDRY_STORAGE.with(|service| service.borrow_mut().get_mut(&payload.laundry_id)) {
                Some(mut laundry) => {
                    if laundry.status == "paid/on progress".to_string() || laundry.status == "paid/done".to_string() {
                        return Err(Error::AlreadyPaid { msg: "Laundry already paid".to_string() });
                    }
                    laundry.status = "paid/on progress".to_string();

                    let current_timestamp = time();
                    let regular_time = 86400000000000 + current_timestamp;
                    let express_time = 14400000000000 + current_timestamp;
                    let finish: u64 = match laundry.package.as_str() {
                        "regular" => regular_time,
                        "express" => express_time,
                        _ => 0,
                    };

                    laundry.finished_at = Some(finish);
                    laundry.updated_at = Some(time());
                    Ok(laundry.clone())
                }
                None => Err(Error::NotFound { msg: "Laundry not found".to_string() })
            }
        }
        None => Err(Error::NotFound { msg: "User not found".to_string() })
    }
}

// Function to check if a laundry is done
#[ic_cdk::update]
fn is_laundry_done(id: u64) -> Result<Laundry, Error> {
    // Retrieve laundry by ID
    match get_laundry(&id) {
        Some(mut laundry) => {
            // Check if laundry is already marked as done
            if laundry.status == "paid/done".to_string() {
                return Err(Error::LaundryAlreadyDone {
                    msg: "Laundry is already marked as done".to_string(),
                });
            }

            // Check if laundry has a finish time
            if let Some(finish) = laundry.finished_at {
                if time() > finish {
                    // If laundry is done, update status and timestamps
                    laundry.status = "paid/done".to_string();
                    laundry.updated_at = Some(time());
                    do_insert_laundry(&laundry);

                    // Update user's completed orders
                    match USER_STORAGE.with(|service| service.borrow().get(&laundry.user_id)) {
                        Some(mut user) => {
                            user.completed_orders.push(laundry.id);
                            user.active_orders.retain(|&x| x != laundry.id);
                            do_insert_user(&user);
                        }
                        None => {
                            return Err(Error::NotFound {
                                msg: "User not found".to_string(),
                            });
                        }
                    }

                    return Ok(laundry);
                } else {
                    // Calculate time left for laundry completion
                    let duration = finish - time();
                    let hours = duration / 3600000000000;
                    let minutes = (duration - (hours * 3600000000000)) / 60000000000;
                    return Err(Error::LaundryNotDone {
                        msg: format!("Laundry not done. Time left: {}h {}m ", hours, minutes),
                        remaining_time: (hours, minutes), // Include remaining time
                    });
                }
            } else {
                
                // Return error if laundry has no finish time
            return Err(Error::InvalidInput {
                    msg: "Laundry has no finish time".to_string(),
                });
            }
        }
        None => Err(Error::NotFound {
            msg: "Laundry not found".to_string(),
        }),
    }
}

// Export Candid functions
ic_cdk::export_candid!();
