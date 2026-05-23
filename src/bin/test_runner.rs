use openclaw_swarm::runners::openclaw_runner::{OpenClawRunner, FileOp};

fn main() {
    let runner = OpenClawRunner::new(
        r"C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm",
        30
    );
    
    println!("=== TEST 1: memory_search ===");
    match runner.memory_search("agent reasoning LLM") {
        Ok(result) => {
            println!("Action: {}", result.action);
            println!("Success: {}", result.success);
            println!("Output preview:\n{}", &result.output[..result.output.len().min(800)]);
        }
        Err(e) => println!("ERROR: {}", e),
    }
    
    println!("\n=== TEST 2: file_op (read) ===");
    match runner.file_op(FileOp::Read { 
        path: "Cargo.toml".to_string() 
    }) {
        Ok(result) => {
            println!("Action: {}", result.action);
            println!("Success: {}", result.success);
            println!("Output preview:\n{}", &result.output[..result.output.len().min(300)]);
        }
        Err(e) => println!("ERROR: {}", e),
    }
    
    println!("\n=== TEST 3: exec_command ===");
    match runner.exec_command("git", &["status", "--short"]) {
        Ok(result) => {
            println!("Action: {}", result.action);
            println!("Success: {}", result.success);
            println!("Output:\n{}", result.output);
        }
        Err(e) => println!("ERROR: {}", e),
    }
    
    println!("\n=== TEST 4: execute_bridge (enqueues to queue) ===");
    // This requires an async runtime - skip in sync test runner
    println!("SKIPPED: execute_bridge requires tokio runtime");
    
    println!("\nAll tests completed.");
}
