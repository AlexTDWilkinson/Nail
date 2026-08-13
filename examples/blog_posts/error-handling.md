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
// string_length CANNOT fail - no !e in its return type
name_length:i = string_length(`Nail`);

// io_read_line_prompt CAN fail - it returns s!e. A result can never
// sit in a variable, so it must be handled right at the call
user_input:s = danger(io_read_line_prompt(`Enter your name: `));
```

## Three Ways to Handle Errors

### 1. The `danger` Approach: Living on the Edge

When you're confident a function won't fail, use `danger`:

```nail
// Reading a config file that MUST exist
config:s = danger(fs_read(`config.nail`));

// Converting a number we KNOW is valid
port:i = danger(int_from(`3000`));
```

If the operation fails, the program crashes with the underlying error message.

### 2. The `safe` Approach: Graceful Handling

When you want to handle errors gracefully, use `safe`. Its second argument is a named function that takes the error (type `e`) and produces the fallback:

```nail-fragment
// Provide a default value if parsing fails
f default_age(error_msg:e):i {
    print(`Invalid age provided, using default`);
    r 0;
}
user_age:i = safe(int_from(age_input), default_age);

// Try multiple servers until one works
f all_servers_down(error_msg:e):HTTP_Response {
    r HTTP_Response {
        status = 503,
        body = `{"error": "All servers down"}`,
        content_type = `application/json`,
        headers = hashmap_new()
    };
}
f try_backup(error_msg:e):HTTP_Response {
    // Primary failed, try backup
    r safe(
        http_request(HTTP_Method::Get, `https://backup.api.com/data`, hashmap_new(), ``),
        all_servers_down
    );
}
response:HTTP_Response = safe(
    http_request(HTTP_Method::Get, `https://primary.api.com/data`, hashmap_new(), ``),
    try_backup
);
```

### 3. The `expect` Approach: This Can Never Fail

`expect` unwraps exactly like `danger` and takes only the result. The difference is intent: `danger` marks a risk you accept, `expect` marks a case you believe impossible, crash loudly if you are wrong:

```nail
// A local database the app cannot start without -
// if this fails, something is deeply wrong, so crash loudly
database:DB_Postgres = expect(db_postgres_connect(`postgres://localhost/myapp`));
```

## Real-World Example: User Registration

```nail-fragment
struct User {
    username:s,
    email:s,
    age:i
}

// A failed parse becomes -1, which the age range check rejects
f unparseable_age(parse_error:e):i {
    print(`Invalid age:`, parse_error);
    r -1;
}

// A failed lookup means no stored user, so the name is free
f missing_user(db_error:e):User {
    r User { username = ``, email = ``, age = 0 };
}

f register_user(username_input:s, email_input:s, age_input:s):User!e {
    // Validate username
    if {
        string_length(username_input) < 3 -> {
            r e(`Username must be at least 3 characters`);
        }
    };
    
    // Validate email
    if {
        !string_contains(email_input, `@`) -> {
            r e(`Invalid email format`);
        }
    };
    
    // Parse age - this could fail
    age:i = safe(int_from(age_input), unparseable_age);
    
    // Check age range
    if {
        age < 13 -> { r e(`Must be 13 or older to register`); },
        age > 120 -> { r e(`Invalid age provided`); }
    };
    
    // Check if username already exists
    existing_user:User = safe(db_find_user(username_input), missing_user);
    if {
        existing_user.username == username_input -> { r e(`Username already taken`); }
    };
    
    // Create user
    new_user:User = User {
        username = username_input,
        email = email_input,
        age = age
    };
    
    // Save to database
    saved_user:User = danger(db_save_user(new_user));
    
    r saved_user;
}

// Usage
f guest_user(error_msg:e):User {
    print(`Registration failed:`, error_msg);
    r User { 
        username = `guest`, 
        email = `guest@example.com`, 
        age = 0 
    };
}
user:User = safe(register_user(`alice`, `alice@example.com`, `25`), guest_user);
```

## Error Context and Tracing

Nail's error messages carry context: stdlib failures name the function that failed and echo the offending input, and your own `e(...)` messages should do the same:

```nail-fragment
f process_order(order_id:s):Receipt!e {
    // Each danger() unwraps its result, and crashes with the
    // underlying error message if that step fails
    order:Order = danger(fetch_order(order_id));
    
    payment:Payment = danger(process_payment(order));
    
    receipt:Receipt = danger(generate_receipt(payment));
    
    r receipt;
}
```

## Parallel Error Handling

Even in parallel blocks, errors are handled properly:

```nail-fragment
f fallback_preferences(error_msg:e):Preferences {
    r default_preferences();
}

p
    user_data:User = danger(fetch_user(user_id));
    preferences:Preferences = safe(fetch_preferences(user_id), fallback_preferences);
    notifications:a:Notification = danger(fetch_notifications(user_id));
/p
// All three operations run in parallel
// If any danger() call fails, the program crashes with that error
```

## Best Practices

### 1. Use the Right Tool

- **`danger`**: When failure means a bug in your code
- **`safe`**: When failure is expected and you can recover
- **`expect`**: When failure should be impossible, and you want the crash to be loud if you are wrong

### 2. Fail Fast in Development

```nail-fragment
f fallback_config(error_msg:e):Config {
    r default_config();
}

// During development
debug_mode:b = true;
config:Config = if {
    debug_mode -> { r danger(load_config()); },
    else -> { r safe(load_config(), fallback_config); }
};
```

### 3. Provide Context in Custom Errors

```nail
f validate_price(price:f):f!e {
    if {
        price < 0.0 -> { 
            r e(array_join([
                `Invalid price: `, 
                string_from(price), 
                `. Price must be non-negative`
            ], ``)); 
        },
        price > 1000000.0 -> { 
            r e(`Price exceeds maximum allowed value of 1,000,000`); 
        }
    };
    r price;
}
```

## Why This Design?

1. **No Hidden Failures**: Every function that can fail says so in its type
2. **Explicit Handling**: You must choose how to handle errors
3. **Messages With Context**: Runtime errors name the failing function and echo the offending input
4. **Type Safety**: The compiler ensures all errors are handled
5. **Performance**: Zero-cost abstractions when transpiled to Rust

## Conclusion

Nail's error handling forces you to think about failure cases upfront, resulting in more robust code. By making errors explicit in the type system and providing clear handling mechanisms, Nail eliminates entire categories of bugs while keeping your code clean and maintainable.

Remember: In Nail, **errors are values, not surprises**! 🔨