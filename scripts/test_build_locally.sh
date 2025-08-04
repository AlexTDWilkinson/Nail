#!/bin/bash
# Test the build script locally before pushing to GitHub

echo "Testing build script locally..."
./scripts/build_for_render.sh

if [ -f "./nail-website" ]; then
    echo "✅ Build successful! Binary created: ./nail-website"
    echo "You can test it with: ./nail-website"
    echo ""
    echo "To clean up: rm ./nail-website"
else
    echo "❌ Build failed - no binary created"
    exit 1
fi