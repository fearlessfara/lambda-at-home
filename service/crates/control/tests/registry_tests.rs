use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[test]
fn test_function_deletion_state_management() {
    // Test the core deletion state management functionality
    let functions_being_deleted = Arc::new(Mutex::new(HashSet::new()));
    
    // Initially, no functions should be marked for deletion
    assert!(!is_function_being_deleted(&functions_being_deleted, "test-function"));
    
    // Mark a function for deletion
    mark_function_for_deletion(&functions_being_deleted, "test-function");
    assert!(is_function_being_deleted(&functions_being_deleted, "test-function"));
    
    // Mark another function for deletion
    mark_function_for_deletion(&functions_being_deleted, "another-function");
    assert!(is_function_being_deleted(&functions_being_deleted, "another-function"));
    assert!(is_function_being_deleted(&functions_being_deleted, "test-function"));
    
    // Unmark a function
    unmark_function_for_deletion(&functions_being_deleted, "test-function");
    assert!(!is_function_being_deleted(&functions_being_deleted, "test-function"));
    assert!(is_function_being_deleted(&functions_being_deleted, "another-function"));
    
    // Unmark the other function
    unmark_function_for_deletion(&functions_being_deleted, "another-function");
    assert!(!is_function_being_deleted(&functions_being_deleted, "another-function"));
}

#[test]
fn test_multiple_functions_deletion_isolation() {
    let functions_being_deleted = Arc::new(Mutex::new(HashSet::new()));
    
    // Mark function1 for deletion
    mark_function_for_deletion(&functions_being_deleted, "function1");
    assert!(is_function_being_deleted(&functions_being_deleted, "function1"));
    assert!(!is_function_being_deleted(&functions_being_deleted, "function2"));
    
    // Mark function2 for deletion
    mark_function_for_deletion(&functions_being_deleted, "function2");
    assert!(is_function_being_deleted(&functions_being_deleted, "function1"));
    assert!(is_function_being_deleted(&functions_being_deleted, "function2"));
    
    // Unmark only function1
    unmark_function_for_deletion(&functions_being_deleted, "function1");
    assert!(!is_function_being_deleted(&functions_being_deleted, "function1"));
    assert!(is_function_being_deleted(&functions_being_deleted, "function2"));
}

#[test]
fn test_deletion_state_thread_safety() {
    use std::thread;
    
    let functions_being_deleted = Arc::new(Mutex::new(HashSet::new()));
    let mut handles = Vec::new();
    
    // Spawn multiple threads to test thread safety
    for i in 0..10 {
        let functions_being_deleted = functions_being_deleted.clone();
        let handle = thread::spawn(move || {
            let function_name = format!("function-{}", i);
            mark_function_for_deletion(&functions_being_deleted, &function_name);
            assert!(is_function_being_deleted(&functions_being_deleted, &function_name));
            unmark_function_for_deletion(&functions_being_deleted, &function_name);
            assert!(!is_function_being_deleted(&functions_being_deleted, &function_name));
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    // All functions should be unmarked
    for i in 0..10 {
        assert!(!is_function_being_deleted(&functions_being_deleted, &format!("function-{}", i)));
    }
}

// Helper functions that mirror the ControlPlane methods
fn mark_function_for_deletion(functions_being_deleted: &Arc<Mutex<HashSet<String>>>, function_name: &str) {
    if let Ok(mut set) = functions_being_deleted.lock() {
        set.insert(function_name.to_string());
    }
}

fn unmark_function_for_deletion(functions_being_deleted: &Arc<Mutex<HashSet<String>>>, function_name: &str) {
    if let Ok(mut set) = functions_being_deleted.lock() {
        set.remove(function_name);
    }
}

fn is_function_being_deleted(functions_being_deleted: &Arc<Mutex<HashSet<String>>>, function_name: &str) -> bool {
    if let Ok(set) = functions_being_deleted.lock() {
        set.contains(function_name)
    } else {
        false
    }
}

