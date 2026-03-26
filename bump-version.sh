#!/bin/bash
# bump-version.sh
RAW_VERSION=$1

if [ -z "$RAW_VERSION" ]; then
  echo "Usage: ./bump-version.sh <version> (e.g., 0.1.1 or v0.1.1)"
  exit 1
fi

VERSION=${RAW_VERSION#v}
TAG="v$VERSION"

echo "Updating versions to $VERSION and creating tag $TAG..."

# 1. Update Rust Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' "0,/version = \".*\"/s//version = \"$VERSION\"/" Cargo.toml
else
  sed -i "0,/version = \".*\"/s//version = \"$VERSION\"/" Cargo.toml
fi

# 2. Update Frontend package.json
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" frontend/package.json
else
  sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" frontend/package.json
fi

# 3. Update Docs package.json
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" docs/package.json
else
  sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" docs/package.json
fi

# 4. Git tags
echo "Committing and tagging..."
git add Cargo.toml frontend/package.json docs/package.json
git commit -m "chore: bump version to $TAG"
git tag "$TAG"

echo "--------------------------------------------------------"
echo "Done! Version updated to $VERSION and tag $TAG created."
echo "To trigger the release, run:"
echo "  git push origin main --tags"
echo "--------------------------------------------------------"
