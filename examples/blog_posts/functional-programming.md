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
increased_prices:a:f = map_float(prices, |price:f|:f { r price * 1.1; });
```

### Filter: Select What You Need

```nail
// Get all premium users
users:a:User = fetch_all_users();
premium_users:a:User = filter_struct(users, |user:User|:b { 
    r user.subscription_level == `premium`; 
});

// Chain operations by breaking them down
active_premium_users:a:User = filter_struct(premium_users, |user:User|:b {
    r user.last_login_days < 30;
});
```

### Reduce: Combine Into One

```nail
// Calculate total revenue
orders:a:Order = get_todays_orders();
total_revenue:f = reduce_struct(orders, 0.0, |sum:f, order:Order|:f {
    r sum + order.total;
});

// Find the oldest user
users:a:User = get_all_users();
oldest_user:User = reduce_struct(users, users[0], |oldest:User, current:User|:User {
    if {
        current.age > oldest.age => { r current; },
        else => { r oldest; }
    };
});
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
process_logs:s = dangerous(fs_read(`/var/log/app.log`));
log_lines:a:s = string_split(process_logs, `\n`);

// Parse each line into structured data
log_entries:a:LogEntry = map_string(log_lines, |line:s|:LogEntry {
    parts:a:s = string_split(line, `,`);
    r LogEntry {
        timestamp: dangerous(int_from(array_get(parts, 0))),
        user_id: dangerous(array_get(parts, 1)),
        action: dangerous(array_get(parts, 2)),
        duration_ms: dangerous(int_from(array_get(parts, 3)))
    };
});

// Filter for slow requests
slow_requests:a:LogEntry = filter_struct(log_entries, |entry:LogEntry|:b {
    r entry.duration_ms > 1000;
});

// Calculate average duration of slow requests
total_duration:i = reduce_struct(slow_requests, 0, |sum:i, entry:LogEntry|:i {
    r sum + entry.duration_ms;
});
avg_duration:f = float_from(total_duration) / float_from(array_len(slow_requests));

print(string_concat([`Average slow request duration: `, string_from(avg_duration), `ms`]));
```

## Advanced Patterns

### Parallel Processing with map_parallel

```nail
// Process images in parallel
image_paths:a:s = get_image_paths();
processed_images:a:ProcessedImage = map_parallel_string(image_paths, |path:s|:ProcessedImage {
    image_data:Bytes = dangerous(fs_read_bytes(path));
    thumbnail:Bytes = generate_thumbnail(image_data);
    metadata:ImageMetadata = extract_metadata(image_data);
    
    r ProcessedImage {
        original_path: path,
        thumbnail: thumbnail,
        metadata: metadata
    };
});
```

### Building Complex Aggregations

```nail
struct Sales {
    product_id:s,
    quantity:i,
    price:f,
    region:s
}

// Group sales by region using reduce
sales_data:a:Sales = get_quarterly_sales();
regional_totals:HashMap = reduce_struct(sales_data, hashmap_new(), 
    |totals:HashMap, sale:Sales|:HashMap {
        current_total:f = safe(hashmap_get(totals, sale.region), |e|:f { r 0.0; });
        sale_value:f = float_from(sale.quantity) * sale.price;
        new_total:f = current_total + sale_value;
        r hashmap_insert(totals, sale.region, new_total);
    }
);
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