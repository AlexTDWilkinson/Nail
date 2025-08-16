# Parallel Blocks: Effortless Concurrency in Nail

One of Nail's killer features is the `parallel` block - a simple way to run multiple operations concurrently without dealing with threads, locks, or race conditions. Let's explore how Nail makes concurrent programming accessible to everyone.

## The Problem with Traditional Concurrency

In most languages, concurrent programming involves:
- Complex thread management
- Mutex locks and deadlocks
- Race conditions
- Callback hell or Promise chains
- Difficult debugging

Nail's solution? Just write `p }`.

## Basic Parallel Blocks

```nail
// Sequential approach - slow!
user:User = fetch_user_from_db(user_id);        // 100ms
posts:a:Post = fetch_user_posts(user_id);       // 150ms  
friends:a:User = fetch_user_friends(user_id);   // 120ms
// Total time: 370ms

// Parallel approach - fast!
p
    user:User = fetch_user_from_db(user_id);      // (
    posts:a:Post = fetch_user_posts(user_id);     // ( All run
    friends:a:User = fetch_user_friends(user_id);  // ( together!
}
// Total time: 150ms (the slowest operation)
```

## Real-World Example: Building a Dashboard

```nail
struct DashboardData {
    user_info:UserInfo,
    recent_activity:a:Activity,
    notifications:a:Notification,
    statistics:Stats,
    recommendations:a:Item
}

f build_user_dashboard(user_id:s):DashboardData {
    // Fetch all data in parallel - turns 5 sequential API calls into 1 concurrent operation
    p
        user_info:UserInfo = danger(fetch_user_info(user_id));
        recent_activity:a:Activity = safe(
            fetch_recent_activity(user_id),
            (e:s):a:Activity { r []; }  // Return empty array on error
        );
        notifications:a:Notification = danger(fetch_notifications(user_id));
        statistics:Stats = danger(calculate_user_stats(user_id));
        recommendations:a:Item = safe(
            fetch_recommendations(user_id),
            (e:s):a:Item { r get_default_recommendations(); }
        );
    }
    
    // All variables are available here after parallel execution
    r DashboardData {
        user_info: user_info,
        recent_activity: recent_activity,
        notifications: notifications,
        statistics: statistics,
        recommendations: recommendations
    };
}
```

## Advanced Pattern: Parallel Data Processing

```nail
struct ImageResult {
    path:s,
    thumbnail:s,
    metadata:ImageMeta,
    upload_url:s
}

f process_uploaded_images(image_paths:a:s):a:ImageResult {
    // Process each image in parallel
    results:a:ImageResult = [];
    
    // Split into chunks for batch processing
    chunk_size:i = 5;
    chunks:a:a:s = array_chunk(image_paths, chunk_size);
    
    // Process each chunk
    each_array(chunks, (chunk:a:s):v {
        p
            result1:ImageResult = process_single_image(array_get(chunk, 0));
            result2:ImageResult = process_single_image(array_get(chunk, 1));
            result3:ImageResult = process_single_image(array_get(chunk, 2));
            result4:ImageResult = process_single_image(array_get(chunk, 3));
            result5:ImageResult = process_single_image(array_get(chunk, 4));
        }
        
        // Add results to our collection
        results = array_concat(results, [result1, result2, result3, result4, result5]);
    });
    
    r results;
}

f process_single_image(path:s):ImageResult {
    p
        // All these operations happen concurrently for each image
        thumbnail_path:s = generate_thumbnail(path);
        metadata:ImageMeta = extract_image_metadata(path);
        upload_url:s = upload_to_cdn(path);
    }
    
    r ImageResult {
        path: path,
        thumbnail: thumbnail_path,
        metadata: metadata,
        upload_url: upload_url
    };
}
```

## Parallel API Aggregation

```nail
struct PriceComparison {
    amazon_price:f,
    ebay_price:f,
    walmart_price:f,
    best_price:f,
    best_vendor:s
}

f compare_prices(product_id:s):PriceComparison {
    // Query multiple APIs simultaneously
    p
        amazon:f = safe(
            fetch_amazon_price(product_id),
            (e:s):f { r 999999.99; }  // High default if API fails
        );
        ebay:f = safe(
            fetch_ebay_price(product_id),
            (e:s):f { r 999999.99; }
        );
        walmart:f = safe(
            fetch_walmart_price(product_id),
            (e:s):f { r 999999.99; }
        );
    }
    
    // Determine best price
    best_price:f = float_min(float_min(amazon, ebay), walmart);
    best_vendor:s = if {
        best_price == amazon => { `Amazon`; },
        best_price == ebay => { `eBay`; },
        best_price == walmart => { `Walmart`; },
        else => { `Unknown`; }
    };
    
    r PriceComparison {
        amazon_price: amazon,
        ebay_price: ebay,
        walmart_price: walmart,
        best_price: best_price,
        best_vendor: best_vendor
    };
}
```

## Parallel File Processing

```nail
f analyze_log_files(log_dir:s):LogAnalysis {
    // Get all log files
    log_files:a:s = danger(fs_list_files(log_dir));
    
    // Process files in parallel batches
    p
        error_count:i = count_errors_in_files(log_files);
        warning_count:i = count_warnings_in_files(log_files);
        unique_ips:a:s = extract_unique_ips(log_files);
        peak_hour:i = find_peak_traffic_hour(log_files);
        total_requests:i = count_total_requests(log_files);
    }
    
    r LogAnalysis {
        errors: error_count,
        warnings: warning_count,
        unique_visitors: array_length(unique_ips),
        peak_hour: peak_hour,
        total_requests: total_requests
    };
}
```

## Combining Parallel with Functional Operations

```nail
struct WebsiteCheck {
    url:s,
    status_code:i,
    response_time_ms:i,
    is_healthy:b
}

f monitor_websites(urls:a:s):a:WebsiteCheck {
    // Check 10 websites at a time
    checks:a:WebsiteCheck = [];
    
    // Process in batches of 10
    url_batches:a:a:s = array_chunk(urls, 10);
    
    each_array(url_batches, (batch:a:s):v {
        // Check all URLs in this batch simultaneously
        p
            check0:WebsiteCheck = check_website(array_get(batch, 0));
            check1:WebsiteCheck = check_website(array_get(batch, 1));
            check2:WebsiteCheck = check_website(array_get(batch, 2));
            check3:WebsiteCheck = check_website(array_get(batch, 3));
            check4:WebsiteCheck = check_website(array_get(batch, 4));
            check5:WebsiteCheck = check_website(array_get(batch, 5));
            check6:WebsiteCheck = check_website(array_get(batch, 6));
            check7:WebsiteCheck = check_website(array_get(batch, 7));
            check8:WebsiteCheck = check_website(array_get(batch, 8));
            check9:WebsiteCheck = check_website(array_get(batch, 9));
        }
        
        // Collect results
        batch_results:a:WebsiteCheck = [
            check0, check1, check2, check3, check4,
            check5, check6, check7, check8, check9
        ];
        
        checks = array_concat(checks, batch_results);
    });
    
    r checks;
}
```

## Performance Tips

### 1. Group Related Operations
```nail
// Good - related data fetched together
p
    user:User = fetch_user(id);
    profile:Profile = fetch_profile(id);
    settings:Settings = fetch_settings(id);
}

// Bad - unrelated operations
p
    user:User = fetch_user(id);
    weather:Weather = fetch_weather();
    stock_price:f = fetch_stock_price(`AAPL`);
}
```

### 2. Balance Granularity
```nail
// Too fine-grained (overhead)
each_int([1,2,3,4,5], (n:i):v {
    p
        result:i = n * 2;
    }
});

// Better - batch operations
numbers:a:i = range(1, 1000);
chunks:a:a:i = array_chunk(numbers, 100);
results:a:i = map_array(chunks, (chunk:a:i):a:i {
    p
        part1:a:i = process_chunk(array_slice(chunk, 0, 50));
        part2:a:i = process_chunk(array_slice(chunk, 50, 100));
    }
    r array_concat(part1, part2);
});
```

### 3. Handle Errors Appropriately
```nail
p
    // Critical operation - use danger
    user:User = danger(fetch_user(user_id));
    
    // Optional enhancement - use safe
    recommendations:a:Item = safe(
        fetch_recommendations(user_id),
        (e:s):a:Item { r []; }
    );
}
```

## Under the Hood

When Nail transpiles to Rust, parallel blocks become:
- `tokio::join!` for concurrent async operations
- Automatic `async/await` handling
- Proper error propagation
- Zero-cost abstractions

## Why Parallel Blocks Matter

1. **Simplicity**: No threads, no locks, just `p` and `\p`
2. **Safety**: No race conditions or data races
3. **Performance**: Automatic optimization for multi-core systems
4. **Clarity**: Intent is obvious from the code structure
5. **Composability**: Parallel blocks can be nested and combined

## Conclusion

Parallel blocks in Nail make concurrent programming accessible to everyone. By removing the complexity of traditional threading models and providing a simple, safe syntax, Nail lets you write fast, concurrent code without the headaches.

Remember: When operations can run independently, wrap them in `p }` and watch your performance soar! 🚀🔨