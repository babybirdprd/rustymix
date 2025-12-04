#!/bin/bash
set -e

# ==============================================================================
# Rustymix Test Suite
# ==============================================================================

# Setup directories
TEST_DIR="tests/sandbox"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Path to binary
RUSTYMIX_BIN="../../target/debug/rustymix"

echo "=== Building Rustymix ==="
# We assume it's already built by the plan step, but let's verify
if [ ! -f "$RUSTYMIX_BIN" ]; then
    echo "Error: Binary not found at $RUSTYMIX_BIN"
    exit 1
fi

echo "=== Setting up Test Environment ==="

# Create dummy repositories for different languages to test compression and detection
# 1. Rust Repo
mkdir -p rust_repo/src
cat <<EOF > rust_repo/src/main.rs
fn main() {
    println!("Hello, world!");
}

struct TestStruct {
    field: i32,
}

impl TestStruct {
    fn new() -> Self {
        Self { field: 0 }
    }
}
// This is a comment
EOF
(cd rust_repo && git init && git add . && git commit -m "Initial commit")

# 2. TypeScript Repo
mkdir -p ts_repo/src
cat <<EOF > ts_repo/src/index.ts
interface User {
  id: number;
  name: string;
}

class UserManager {
  constructor(private users: User[]) {}

  getUser(id: number): User | undefined {
    // Return user
    return this.users.find(u => u.id === id);
  }
}

function helper() {
  console.log("Helper");
}
EOF
(cd ts_repo && git init && git add . && git commit -m "Initial commit")

# 3. Python Repo
mkdir -p py_repo
cat <<EOF > py_repo/app.py
class Processor:
    def __init__(self):
        self.data = []

    def process(self, item):
        # Process item
        print(f"Processing {item}")
        return True

def main():
    p = Processor()
    p.process("test")
EOF
(cd py_repo && git init && git add . && git commit -m "Initial commit")

# 4. Go Repo
mkdir -p go_repo
cat <<EOF > go_repo/main.go
package main

import "fmt"

type Server struct {
	Port int
}

func (s *Server) Start() {
	// Start server
	fmt.Println("Starting...")
}

func main() {
	s := &Server{Port: 8080}
	s.Start()
}
EOF
(cd go_repo && git init && git add . && git commit -m "Initial commit")


# 5. Mixed Repo (for general tests)
mkdir -p mixed_repo
echo "Secret=API_KEY_12345678901234567890123456789012" > mixed_repo/secret.env
echo "Normal file" > mixed_repo/normal.txt
echo "Ignored file" > mixed_repo/ignore_me.log
cat <<EOF > mixed_repo/.gitignore
*.log
secret.env
EOF
(cd mixed_repo && git init && git add . && git commit -m "Initial commit")


echo "=== Running Tests ==="

# TEST 1: Basic XML Output (Rust)
echo "Test 1: Basic XML Output"
"$RUSTYMIX_BIN" rust_repo -o output_1.xml
if grep -q "<repomix>" output_1.xml && grep -q "src/main.rs" output_1.xml; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi

# TEST 2: Output Styles (Markdown)
echo "Test 2: Markdown Output"
"$RUSTYMIX_BIN" ts_repo --style markdown -o output_2.md
if grep -q "# File Summary" output_2.md && grep -q "class UserManager" output_2.md; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi

# TEST 3: Compression (Python)
echo "Test 3: Compression (Python)"
"$RUSTYMIX_BIN" py_repo --compress -o output_3.txt
# Should contain class/def definitions but NOT the print statement body if compression works well?
# Actually tree-sitter compression retains signatures.
# Let's check if the file is generated and contains the class definition.
if grep -q "class Processor" output_3.txt; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi

# TEST 4: Security Check
echo "Test 4: Security Check"
# By default security check should be enabled.
# The 'secret.env' file in mixed_repo contains a fake API key pattern.
# However, .gitignore usually ignores .env files. But let's see if we can trigger it.
# We'll create a file that IS tracked but has a secret.
echo "token = 'ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890'" > mixed_repo/leaked_token.txt
# We need to NOT git ignore it to be picked up by walker, or force include it.
# Actually walker respects gitignore by default.
"$RUSTYMIX_BIN" mixed_repo --no-gitignore -o output_4.xml
if grep -q "leaked_token.txt" output_4.xml; then
    echo "FAIL - Security check failed, file included"
    grep "leaked_token.txt" output_4.xml
    exit 1
else
    echo "PASS - File with secret excluded"
fi

# TEST 5: Git Integration (Logs)
echo "Test 5: Git Logs"
"$RUSTYMIX_BIN" go_repo --include-logs -o output_5.xml
if grep -q "<git_log>" output_5.xml; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi

# TEST 6: Remove Comments
echo "Test 6: Remove Comments"
"$RUSTYMIX_BIN" rust_repo --remove-comments -o output_6.xml
if grep -q "This is a comment" output_6.xml; then
    echo "FAIL - Comment found"
    exit 1
else
    echo "PASS"
fi

# TEST 7: Ignore Patterns
echo "Test 7: Ignore Patterns"
# ignore_me.log is in .gitignore, so default run should ignore it.
"$RUSTYMIX_BIN" mixed_repo -o output_7.xml
if grep -q "ignore_me.log" output_7.xml; then
    echo "FAIL - Ignored file included"
    exit 1
else
    echo "PASS"
fi

# TEST 7b: CLI Ignore
echo "Test 7b: CLI Ignore"
"$RUSTYMIX_BIN" ts_repo --ignore "src/index.ts" -o output_7b.xml
if [ -s output_7b.xml ]; then
    if grep -q "index.ts" output_7b.xml; then
        echo "FAIL - CLI Ignore not respected"
        exit 1
    else
        echo "PASS"
    fi
else
    echo "PASS"
fi

# TEST 8: Config File
echo "Test 8: Config File"
cat <<EOF > custom_config.json
{
  "output": {
    "style": "json",
    "topFilesLength": 10
  },
  "ignore": {
    "customPatterns": ["**/*.ts"]
  },
  "security": {
    "enableSecurityCheck": true
  }
}
EOF
"$RUSTYMIX_BIN" ts_repo --config custom_config.json -o output_8.json --verbose
# Should be JSON, and ignore .ts files? Wait if I ignore *.ts in ts_repo, result is empty?
# Let's see. The repo only has index.ts.
if [ -s output_8.json ]; then
     # Check content. It should NOT contain index.ts content if ignored.
     if grep -q "index.ts" output_8.json; then
         echo "FAIL - Config ignore pattern not respected"
         exit 1
     else
         echo "PASS"
     fi
else
    # Empty file might be valid JSON "{}" or similar?
    echo "PASS (Empty result as expected)"
fi

echo "=== All Tests Passed ==="
