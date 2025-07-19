# Error Handling in Nail: No Exceptions, No Surprises

Nail takes a unique approach to error handling that eliminates runtime surprises while keeping your code clean and maintainable. Let's explore how Nail makes errors impossible to ignore.

## The Problem with Traditional Error Handling

Most languages use one of these approaches:
- **Exceptions**: Can be thrown anywhere, caught anywhere (or nowhere)
- **Error codes**: Easy to ignore, verbose to check
- **Null/undefined**: The billion-dollar mistake

Nail says **no** to all of these.

## The Nail Way: Explicit Error Types

In Nail, functions that can fail must declare it in their type signature:

```nail
// This function CANNOT fail
add:i = add_numbers(5, 3);

// This function CAN fail - note the !e
user_input:s!e = io_read_line_prompt(`Enter your name: `);
```

## Three Ways to Handle Errors

### 1. The `dangerous` Approach: Living on the Edge

When you're confident a function won't fail, use `dangerous`:

```nail
// Reading a config file that MUST exist
config:s = dangerous(fs_read(`config.nail`));

// Converting a number we KNOW is valid
port:i = dangerous(int_from(`3000`));
```

If the operation fails, the error propagates up automatically with context.

### 2. The `safe` Approach: Graceful Handling

When you want to handle errors gracefully, use `safe`:

```nail
// Provide a default value if parsing fails
user_age:i = safe(
    int_from(age_input),
    |error:s|:i { 
        print(`Invalid age provided, using default`);
        r 0; 
    }
);

// Try multiple servers until one works
response:s = safe(
    http_get(`https://primary.api.com/data`),
    |e:s|:s {
        // Primary failed, try backup
        r safe(
            http_get(`https://backup.api.com/data`),
            |e2:s|:s { r `{"error": "All servers down"}`; }
        );
    }
);
```

### 3. The `expect` Approach: Assert with Message

Similar to `dangerous` but with a custom panic message:

```nail
// This SHOULD work, but if not, we want a clear error
database:Database = expect(
    db_connect(`postgresql://localhost/myapp`),
    `Database connection required for application startup`
);
```

## Real-World Example: User Registration

```nail
struct User {
    username:s,
    email:s,
    age:i
}

f register_user(username_input:s, email_input:s, age_input:s):User!e {
    // Validate username
    if {
        string_len(username_input) < 3 => {
            r e(`Username must be at least 3 characters`);
        }
    };
    
    // Validate email
    if {
        !string_contains(email_input, `@`) => {
            r e(`Invalid email format`);
        }
    };
    
    // Parse age - this could fail
    age:i = safe(
        int_from(age_input),
        |parse_error:s|:i!e { 
            r e(string_concat([`Invalid age: `, parse_error])); 
        }
    );
    
    // Check age range
    if {
        age < 13 => { r e(`Must be 13 or older to register`); },
        age > 120 => { r e(`Invalid age provided`); }
    };
    
    // Check if username already exists
    existing_user:User!e = db_find_user(username_input);
    existing_check:b = safe(
        existing_user,
        |db_error:s|:b { r false; } // No user found is good!
    );
    
    if {
        existing_check => { r e(`Username already taken`); }
    };
    
    // Create user
    new_user:User = User {
        username: username_input,
        email: email_input,
        age: age
    };
    
    // Save to database
    saved_user:User = dangerous(db_save_user(new_user));
    
    r saved_user;
}

// Usage
user_result:User!e = register_user(`alice`, `alice@example.com`, `25`);
user:User = safe(
    user_result,
    |error:s|:User {
        print(string_concat([`Registration failed: `, error]));
        r User { 
            username: `guest`, 
            email: `guest@example.com`, 
            age: 0 
        };
    }
);
```

## Error Context and Tracing

Nail automatically adds context as errors propagate:

```nail
f process_order(order_id:s):Receipt!e {
    order:Order = dangerous(fetch_order(order_id));
    // If fetch_order fails, error includes: "[process_order] Failed to fetch order"
    
    payment:Payment = dangerous(process_payment(order));
    // If process_payment fails: "[process_order] Payment processing failed"
    
    receipt:Receipt = dangerous(generate_receipt(payment));
    // If generate_receipt fails: "[process_order] Receipt generation failed"
    
    r receipt;
}
```

## Parallel Error Handling

Even in parallel blocks, errors are handled properly:

```nail
parallel {
    user_data:User = dangerous(fetch_user(user_id));
    preferences:Preferences = safe(
        fetch_preferences(user_id),
        |e:s|:Preferences { r default_preferences(); }
    );
    notifications:a:Notification = dangerous(fetch_notifications(user_id));
}
// All three operations run in parallel
// If any dangerous() calls fail, the error is caught here
```

## Best Practices

### 1. Use the Right Tool

- **`dangerous`**: When failure means a bug in your code
- **`safe`**: When failure is expected and you can recover
- **`expect`**: When failure is unlikely but you want a clear message

### 2. Fail Fast in Development

```nail
// During development
debug_mode:b = true;
config:Config = if {
    debug_mode => { dangerous(load_config()); },
    else => { 
        safe(load_config(), |e:s|:Config { r default_config(); })
    }
};
```

### 3. Provide Context in Custom Errors

```nail
f validate_price(price:f):f!e {
    if {
        price < 0.0 => { 
            r e(string_concat([
                `Invalid price: `, 
                string_from(price), 
                `. Price must be non-negative`
            ])); 
        },
        price > 1000000.0 => { 
            r e(`Price exceeds maximum allowed value of 1,000,000`); 
        }
    };
    r price;
}
```

## Why This Design?

1. **No Hidden Failures**: Every function that can fail says so in its type
2. **Explicit Handling**: You must choose how to handle errors
3. **Automatic Context**: Error messages accumulate context as they propagate
4. **Type Safety**: The compiler ensures all errors are handled
5. **Performance**: Zero-cost abstractions when transpiled to Rust

## Conclusion

Nail's error handling forces you to think about failure cases upfront, resulting in more robust code. By making errors explicit in the type system and providing clear handling mechanisms, Nail eliminates entire categories of bugs while keeping your code clean and maintainable.

Remember: In Nail, **errors are values, not surprises**! 🔨