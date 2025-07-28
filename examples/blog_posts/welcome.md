# Welcome to Nail: A Language Without Complexity

Nail is a programming language that removes unnecessary complexity while maintaining power and expressiveness. Here's what makes Nail special:

## Key Features

- **No variables** - Everything is immutable by default
- **No loops** - Use functional programming with map, filter, and reduce
- **Automatic parallelism** - Code runs concurrently without explicit threading
- **Built-in async** - All I/O operations are async by default

## Example Code

Here's a simple example that demonstrates Nail's syntax:

```nail
numbers:a:i = [1, 2, 3, 4, 5];
// Using Nail's collection operations
doubled:a:i = map num in numbers {
    y num * 2;
};
sum:i = reduce acc num in doubled from 0 {
    y acc + num;
};
print(sum); // Output: 30
```

## Why Nail?

1. **Simplicity** - Less syntax to learn, fewer ways to make mistakes
2. **Performance** - Automatic parallelization means faster execution
3. **Safety** - Immutability prevents many common bugs
4. **Modern** - Designed for today's multi-core processors

Start your journey with Nail today and experience programming without the complexity!