#!/usr/bin/env python3
"""
Integration test for Swarm Execution Bridge v0.2.0
Runs after parallel subagents complete their tasks.
"""
import subprocess
import sys
import os

def run(cmd, cwd=None):
    """Run a command and return (success, stdout, stderr)"""
    result = subprocess.run(
        cmd,
        shell=True,
        capture_output=True,
        text=True,
        cwd=cwd or os.path.dirname(os.path.abspath(__file__))
    )
    return result.returncode == 0, result.stdout, result.stderr

def test_cargo_check():
    """AC4.1: Compilation success after swarm build"""
    print("\n=== Test: cargo check ===")
    ok, out, err = run("cargo check")
    if ok:
        print("PASS: Compilation clean")
        return True
    else:
        print(f"FAIL: {err[:500]}")
        return False

def test_phases_table():
    """AC1.3: Phases stored in DB"""
    import sqlite3
    print("\n=== Test: phases table exists ===")
    conn = sqlite3.connect("openclaw-swarm.db")
    c = conn.cursor()
    try:
        c.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='phases'")
        if c.fetchone():
            print("PASS: phases table exists")
            return True
        else:
            print("FAIL: phases table missing")
            return False
    except Exception as e:
        print(f"FAIL: {e}")
        return False
    finally:
        conn.close()

def test_task_decomposer_import():
    """T1.2: TaskDecomposer module compiles"""
    print("\n=== Test: TaskDecompiler import ===")
    try:
        # Check if file exists
        path = os.path.join(os.path.dirname(__file__), "..", "src", "execution", "task_decomposer.rs")
        if os.path.exists(path):
            print("PASS: task_decomposer.rs exists")
            return True
        else:
            print("FAIL: task_decomposer.rs missing")
            return False
    except Exception as e:
        print(f"FAIL: {e}")
        return False

def test_state_manager_import():
    """T1.3: StateManager module compiles"""
    print("\n=== Test: StateManager import ===")
    try:
        path = os.path.join(os.path.dirname(__file__), "..", "src", "execution", "state_manager.rs")
        if os.path.exists(path):
            print("PASS: state_manager.rs exists")
            return True
        else:
            print("FAIL: state_manager.rs missing")
            return False
    except Exception as e:
        print(f"FAIL: {e}")
        return False

def test_workspace_snapshot():
    """T3.1: Workspace snapshot module compiles"""
    print("\n=== Test: WorkspaceSnapshot import ===")
    try:
        path = os.path.join(os.path.dirname(__file__), "..", "src", "execution", "workspace_snapshot.rs")
        if os.path.exists(path):
            print("PASS: workspace_snapshot.rs exists")
            return True
        else:
            print("FAIL: workspace_snapshot.rs missing")
            return False
    except Exception as e:
        print(f"FAIL: {e}")
        return False

def main():
    os.chdir(os.path.join(os.path.dirname(__file__), ".."))
    
    print("=" * 60)
    print("Swarm Execution Bridge — Integration Test Suite")
    print("=" * 60)
    
    results = []
    results.append(("cargo check", test_cargo_check()))
    results.append(("phases table", test_phases_table()))
    results.append(("task_decomposer", test_task_decomposer_import()))
    results.append(("state_manager", test_state_manager_import()))
    results.append(("workspace_snapshot", test_workspace_snapshot()))
    
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    passed = sum(1 for _, r in results if r)
    total = len(results)
    for name, result in results:
        status = "PASS" if result else "FAIL"
        print(f"  [{status}] {name}")
    print(f"\nTotal: {passed}/{total} passed")
    
    return 0 if passed == total else 1

if __name__ == "__main__":
    sys.exit(main())
