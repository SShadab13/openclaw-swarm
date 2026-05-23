#!/bin/bash
# GitHub Repo Setup for openclaw-swarm
# Run: chmod +x setup-github.sh && ./setup-github.sh

set -e

echo "=== OpenClaw Swarm GitHub Repo Setup ==="
echo ""
echo "Make sure you're logged into GitHub CLI: gh auth login"
echo ""

# Step 1: Create repo
echo "[1/5] Creating GitHub repository..."
gh repo create SShadab13/openclaw-swarm \
  --public \
  --description "Agent swarm engine with personality-driven MxN matrix architecture" \
  --source=. \
  --remote=origin \
  --push

# Step 2: Files already created via code
echo "[2/5] Files already created in workspace"
echo ""

# Step 3: Initial commit
echo "[3/5] Making initial commit..."
git init
git add .
git commit -m "Initial commit: v0.1.0 agent swarm engine

- Queen orchestrator with personality-driven MxN matrix
- 9 personas + 9 personalities = infinite combinations
- 3 runner modules: Kimi, Claude, OpenClaw
- Sandbox: Git branch isolation
- Letters, Diary, Error Journal
- SQLite persistence for tasks, errors, knowledge"

# Step 4: Push
echo "[4/5] Pushing to GitHub..."
git branch -M main
git push -u origin main

# Step 5: Tag release
echo "[5/5] Tagging v0.1.0..."
git tag -a v0.1.0 -m "Initial release - agent swarm engine"
git push origin v0.1.0

echo ""
echo "=== Done! Visit: https://github.com/SShadab13/openclaw-swarm ==="
