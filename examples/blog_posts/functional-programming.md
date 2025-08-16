# Functional Programming Paradise: Why Nail Replaces Loops with Map, Filter, and Reduce

In Nail, we've made a radical decision: **no loops**. No `for`, no `while`, no `do-while`. Instead, we embrace functional programming patterns that are safer, more expressive, and automatically parallelizable.

## The Problem with Traditional Loops

Traditional loops are error-prone:
- Off-by-one errors
- Infinite loops
- Complex state management
- Difficult to parallelize
- Hard to reason about

## The Nail Way: Functional Iteration

### Map: Transform Every Element

```nail
// Traditional approach (not possible in Nail)
// for (int i = 0; i < prices.length; i++) {
//     prices[i] = prices[i] * 1.1;
// }

// The Nail way - clear, concise, parallelizable
prices:a:f = [99.99, 149.99, 299.99];
increased_prices:a:f = map price in prices {
    y price * 1.1;
};
```

### Filter: Select What You Need

```nail
// Get all premium users
users:a:User = fetch_all_users();
premium_users:a:User = filter user in users {
    y user.subscription_level == `premium`;
};

// Chain operations by breaking them down
active_premium_users:a:User = filter user in premium_users {
    y user.last_login_days < 30;
};
```

### Reduce: Combine Into One

```nail
// Calculate total revenue
orders:a:Order = get_todays_orders();
total_revenue:f = reduce sum order in orders from 0.0 {
    y sum + order.total;
};

// Find the oldest user
users:a:User = get_all_users();
first_user:User = users[0];
oldest_user:User = reduce oldest current in users from first_user {
    if {
        current.age > oldest.age => { y current; },
        else => { y oldest; }
    };
};
```

## Real-World Example: Analytics Pipeline

```nail
struct LogEntry {
    timestamp:i,
    user_id:s,
    action:s,
    duration_ms:i
}

// Process server logs functionally
process_logs:s = danger(fs_read(`/var/log/app.log`));
log_lines:a:s = string_split(process_logs, `\n`);

// Parse each line into structured data
log_entries:a:LogEntry = map line in log_lines {
    parts:a:s = string_split(line, `,`);
    y LogEntry {
        timestamp: danger(int_from(array_get(parts, 0))),
        user_id: danger(array_get(parts, 1)),
        action: danger(array_get(parts, 2)),
        duration_ms: danger(int_from(array_get(parts, 3)))
    };
};

// Filter for slow requests
slow_requests:a:LogEntry = filter entry in log_entries {
    y entry.duration_ms > 1000;
};

// Calculate average duration of slow requests
total_duration:i = reduce sum entry in slow_requests from 0 {
    y sum + entry.duration_ms;
};
avg_duration:f = float_from(total_duration) / float_from(array_length(slow_requests));

print(array_join([`Average slow request duration: `, string_from(avg_duration), `ms`]));
```

## Advanced Patterns

### Parallel Processing with map_parallel

```nail
// Process images in parallel
image_paths:a:s = get_image_paths();
processed_images:a:ProcessedImage = map path in image_paths {
    image_data:Bytes = danger(fs_read_bytes(path));
    thumbnail:Bytes = generate_thumbnail(image_data);
    metadata:ImageMetadata = extract_metadata(image_data);
    
    y ProcessedImage {
        original_path: path,
        thumbnail: thumbnail,
        metadata: metadata
    };
};
```

### Building Complex Aggregations

```nail
struct Sales {
    product_id:s,
    quantity:i,
    price:f,
    region:s
}

// Helper function for safe float conversion
f default_zero(err:e):f {
    print(err);
    r 0.0;
}

// Calculate total sales across all regions
sales_data:a:Sales = get_quarterly_sales();
total_sales:f = reduce sum sale in sales_data from 0.0 {
    quantity:f = safe(float_from(sale.quantity), default_zero);
    sale_value:f = quantity * sale.price;
    y sum + sale_value;
};

// Helper function for when array is empty
f default_sale(err:e):Sales {
    print(err);
    // Return a dummy sale
    r Sales { quantity: 0, price: 0.0, region: `Unknown` };
}

// Or find the highest value sale
first_sale:Sales = safe(array_get(sales_data, 0), default_sale);
highest_sale:Sales = reduce best sale in sales_data from first_sale {
    current_quantity:f = safe(float_from(sale.quantity), default_zero);
    current_value:f = current_quantity * sale.price;
    best_quantity:f = safe(float_from(best.quantity), default_zero);
    best_value:f = best_quantity * best.price;
    if {
        current_value > best_value => { y sale; },
        else => { y best; }
    };
};
```

## Why This Matters

1. **Automatic Parallelization**: The Nail runtime can parallelize `map` operations across cores
2. **No State Bugs**: Each iteration is independent, eliminating shared state issues
3. **Composability**: Functions compose naturally without side effects
4. **Clarity**: The intent is clear - transform, filter, or aggregate
5. **Performance**: Nail's transpilation to Rust ensures zero-overhead abstractions

## Tips for Thinking Functionally

- **Think in transformations**: What do you want to do to each element?
- **Break it down**: Complex loops become simple map→filter→reduce chains
- **Embrace immutability**: Each operation returns a new collection
- **Use the right function**: `map` for transformation, `filter` for selection, `reduce` for aggregation

## Conclusion

By removing loops, Nail forces you to think differently about iteration. The result? Cleaner, safer, and often faster code that's easier to understand and maintain. Welcome to functional programming - Nail style! 🔨