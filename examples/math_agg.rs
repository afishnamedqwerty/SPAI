//! MathOverflow Question Aggregator & Theorem Prover
//!
//! This example demonstrates a multi-agent theorem proving system that:
//! 1. Loads scraped MathOverflow questions from data/mathoverflow/questions/
//! 2. Uses a Proctor agent to dispatch questions to theorem provers
//! 3. Runs a debate workflow with 3 theorem prover agents (different proof styles)
//! 4. Synthesizes the best Lean4 proof and saves to data/mathoverflow/solved/
//!
//! Uses the debate workflow pattern with Lean4 formalization.
//!
//! Run the scraper first: ./tools/mathoverflow_scraper --limit 5

use spai::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use std::process::Command;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Scraped question from MathOverflow
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScrapedQuestion {
    id: String,
    mathoverflow_id: u64,
    url: String,
    title: String,
    body: String,
    tags: String,
    scraped_at: String,
    answer: String,
}

/// Solved question with proof
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SolvedQuestion {
    id: String,
    mathoverflow_id: u64,
    url: String,
    title: String,
    original_question: String,
    formalized_statement: String,
    lean_proof: String,
    informal_proof: String,
    debate_summary: String,
    prover_contributions: ProverContributions,
    lean_verified: bool,
    lean_errors: Option<String>,
    verification_attempts: u32,
    solved_at: String,
}

/// Contributions from each prover
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProverContributions {
    lean_formalist: String,
    constructivist: String,
    classical_reasoner: String,
}

/// Theorem Prover persona types
#[derive(Debug, Clone, Copy)]
enum ProverStyle {
    LeanFormalist,
    Constructivist,
    ClassicalReasoner,
}

impl ProverStyle {
    fn name(&self) -> &str {
        match self {
            ProverStyle::LeanFormalist => "Prof. Ada Typewright (Lean Formalist)",
            ProverStyle::Constructivist => "Dr. Erwin Builder (Constructivist)",
            ProverStyle::ClassicalReasoner => "Prof. Georg Cantor Jr. (Classical Reasoner)",
        }
    }
    
    fn system_prompt(&self) -> &str {
        match self {
            ProverStyle::LeanFormalist => r#"You are Prof. Ada Typewright, a rigorous Lean4 formalist and type theorist.

Your Proof Philosophy:
- All proofs must be expressible in Lean4's type system
- Use ONLY Lean4 core library - NO Mathlib imports (Mathlib is not available)
- Use dependent types, inductive types, and basic tactics
- Prefer explicit term-mode proofs when clarity allows
- Use built-in tactics: simp, rfl, intro, exact, apply, have, show

CRITICAL CONSTRAINTS:
- Do NOT use `import Mathlib.*` - Mathlib is not installed
- Use only Lean4 standard library
- Keep proofs simple and self-contained
- Define any needed helper lemmas inline

Output Format:
```lean4
-- Standalone Lean4 proof (no external imports)

-- Helper definitions if needed
def helper_fn : Type := ...

-- Main theorem
theorem problem_name : <statement> := by
  <tactics>
```

Critique others' proofs for:
- Mathlib dependencies (not allowed!)
- Type correctness
- Missing cases"#,

            ProverStyle::Constructivist => r#"You are Dr. Erwin Builder, an intuitionistic/constructive mathematician.

Your Proof Philosophy:
- Proofs must construct witnesses explicitly
- Avoid classical axioms (Classical.em) unless absolutely necessary
- Prefer algorithms to existence proofs
- Every ∃ should provide a computable witness

CRITICAL CONSTRAINTS:
- Do NOT use `import Mathlib.*` - Mathlib is not installed
- Use only Lean4 standard library
- Keep proofs simple and self-contained

When writing Lean4:
```lean4
-- Constructive proof with explicit witness
theorem exists_example : ∃ n : Nat, n > 0 := ⟨1, Nat.succ_pos 0⟩

-- Avoid Classical.em if possible
```

Critique others' proofs for:
- Mathlib dependencies (not allowed!)
- Unnecessary use of classical axioms
- Non-constructive existence claims"#,

            ProverStyle::ClassicalReasoner => r#"You are Prof. Georg Cantor Jr., a classical mathematician comfortable with powerful axioms.

Your Proof Philosophy:
- The full power of classical logic is acceptable
- Proof by contradiction is a valid technique
- Focus on elegance and brevity

CRITICAL CONSTRAINTS:
- Do NOT use `import Mathlib.*` - Mathlib is not installed
- Use only Lean4 standard library
- Keep proofs simple and self-contained
- Classical axioms from Lean4 core are OK

When writing Lean4:
```lean4
-- Classical proof by contradiction using Lean4 core
theorem classical_proof (p : Prop) [Decidable p] : ¬¬p → p := by
  intro hnnp
  cases Classical.em p with
  | inl h => exact h
  | inr hn => exact absurd hn hnnp
```

Critique others' proofs for:
- Mathlib dependencies (not allowed!)
- Unnecessary complexity
- Missing edge cases"#,
        }
    }
}

/// Proctor agent system prompt
const PROCTOR_SYSTEM_PROMPT: &str = r#"You are the Proof Proctor, a senior mathematician who orchestrates theorem proving debates.

Your Responsibilities:
1. Present mathematical questions clearly to the theorem provers
2. Formalize informal questions into precise mathematical statements
3. Evaluate proof attempts from multiple provers
4. Identify gaps, errors, and strongest arguments
5. Synthesize the best proof from all contributions
6. Produce a final Lean4 proof and informal explanation

CRITICAL CONSTRAINTS FOR LEAN4:
- Do NOT use `import Mathlib.*` - Mathlib is not installed
- Use ONLY Lean4 standard library (core definitions)
- Define all necessary structures, helper functions, and lemmas inline
- The proof must be completely self-contained
- If a complex concept is needed, define a simplified version locally

When Synthesizing Proofs:
- Combine the best elements from each prover
- If provers used Mathlib, REWRITE their logic to use only core Lean4
- Ensure the final Lean4 code is complete and correct
- Provide an informal proof summary for humans

Output Format for Final Synthesis:
```
## Formalized Statement
<precise mathematical statement>

## Lean4 Proof
```lean4
-- Standalone Lean4 proof (no external imports)
<complete Lean4 code with inline definitions>
```

## Informal Proof
<human-readable explanation>

## Debate Summary
<brief summary of key contributions and disagreements>
```"#;

/// Build a theorem prover agent
fn build_prover_agent(
    style: ProverStyle,
    client: Arc<dyn LlmClient>,
) -> anyhow::Result<Agent> {
    Agent::builder()
        .name(style.name())
        .model("anthropic/claude-opus-4.5")
        .system_prompt(style.system_prompt())
        .max_loops(3)
        .temperature(0.7)
        .client(client)
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 2000,
            expose_reasoning: true,
        })
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Build the proctor agent
fn build_proctor_agent(client: Arc<dyn LlmClient>) -> anyhow::Result<Agent> {
    Agent::builder()
        .name("Proof Proctor")
        .model("anthropic/claude-opus-4.5")
        .system_prompt(PROCTOR_SYSTEM_PROMPT)
        .max_loops(5)
        .temperature(0.5)
        .client(client)
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 3000,
            expose_reasoning: true,
        })
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Load questions from data directory
fn load_questions(dir: &Path) -> anyhow::Result<Vec<ScrapedQuestion>> {
    let mut questions = Vec::new();
    
    if !dir.exists() {
        return Err(anyhow::anyhow!(
            "Questions directory not found: {:?}\nRun: ./tools/mathoverflow_scraper --limit 5",
            dir
        ));
    }
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().map_or(false, |e| e == "json") {
            let content = fs::read_to_string(&path)?;
            match serde_json::from_str::<ScrapedQuestion>(&content) {
                Ok(q) => questions.push(q),
                Err(e) => eprintln!("Warning: Failed to parse {:?}: {}", path, e),
            }
        }
    }
    
    questions.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(questions)
}

/// Save solved question
fn save_solved(question: &SolvedQuestion, dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dir)?;
    let filename = format!("{}_solved.json", question.id);
    let path = dir.join(filename);
    let content = serde_json::to_string_pretty(question)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Result of Lean4 verification
#[derive(Debug, Clone)]
struct LeanVerificationResult {
    success: bool,
    errors: Option<String>,
    output: String,
}

/// Check if Lean4 is available on the system
fn check_lean_available() -> bool {
    Command::new("lean")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Verify a Lean4 proof by writing to temp file and running lean
fn verify_lean_proof(proof_code: &str, question_id: &str) -> LeanVerificationResult {
    // Create temp directory for lean files
    let temp_dir = std::env::temp_dir().join("mathoverflow_proofs");
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        return LeanVerificationResult {
            success: false,
            errors: Some(format!("Failed to create temp dir: {}", e)),
            output: String::new(),
        };
    }
    
    // Write proof to temp file
    let proof_file = temp_dir.join(format!("{}.lean", question_id));
    if let Err(e) = fs::write(&proof_file, proof_code) {
        return LeanVerificationResult {
            success: false,
            errors: Some(format!("Failed to write proof file: {}", e)),
            output: String::new(),
        };
    }
    
    println!("🔍 Verifying Lean4 proof: {:?}", proof_file);
    
    // Run lean4 on the file
    let output = Command::new("lean")
        .arg(&proof_file)
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout).to_string();
            let stderr = String::from_utf8_lossy(&result.stderr).to_string();
            let combined_output = format!("{}\n{}", stdout, stderr);
            
            if result.status.success() {
                println!("   ✅ Lean4 verification PASSED");
                LeanVerificationResult {
                    success: true,
                    errors: None,
                    output: combined_output,
                }
            } else {
                // Lean4 outputs errors to stdout, not stderr
                let error_text = if !stdout.trim().is_empty() {
                    stdout.trim().to_string()
                } else {
                    stderr.trim().to_string()
                };
                println!("   ❌ Lean4 verification FAILED");
                println!("   Errors: {}", error_text.lines().take(5).collect::<Vec<_>>().join("\n   "));
                LeanVerificationResult {
                    success: false,
                    errors: Some(error_text),
                    output: combined_output,
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to run lean: {}", e);
            println!("   ⚠️  {}", error_msg);
            LeanVerificationResult {
                success: false,
                errors: Some(error_msg),
                output: String::new(),
            }
        }
    }
}

/// Save the Lean4 proof to a .lean file alongside the JSON
fn save_lean_file(question_id: &str, proof_code: &str, dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dir)?;
    let filename = format!("{}.lean", question_id);
    let path = dir.join(filename);
    fs::write(&path, proof_code)?;
    println!("   📄 Saved Lean file: {:?}", path);
    Ok(())
}

/// Run theorem proving debate for a single question
async fn prove_question(
    question: &ScrapedQuestion,
    provers: &[Agent],
    proctor: &Agent,
    debate_rounds: usize,
) -> anyhow::Result<SolvedQuestion> {
    let start = Instant::now();
    
    println!("\n{}", "═".repeat(80));
    println!("📐 QUESTION: {}", question.title);
    println!("🔗 {}", question.url);
    println!("{}\n", "═".repeat(80));
    
    // Step 1: Proctor formalizes the question
    println!("📋 Proctor formalizing the question...\n");
    
    let formalize_prompt = format!(
        r#"Please formalize this MathOverflow question for theorem proving:

## Title
{}

## Question Body
{}

## Tags
{}

Provide:
1. A precise mathematical statement of what needs to be proven
2. Any necessary definitions or assumptions
3. The mathematical context (what field of math this is from)"#,
        question.title, question.body, question.tags
    );
    
    let formalization = proctor.react_loop(&formalize_prompt).await?;
    println!("📝 Formalization:\n{}\n", formalization.content);
    
    // Step 2: Each prover provides initial proof attempt
    println!("{}", "-".repeat(80));
    println!("🔬 INITIAL PROOF ATTEMPTS\n");
    
    let mut prover_outputs: Vec<String> = Vec::new();
    
    for prover in provers.iter() {
        let proof_prompt = format!(
            r#"Prove the following formalized mathematical statement in Lean4:

{}

Question context: {}

Provide a complete Lean4 proof with:
1. Necessary imports
2. Helper lemmas if needed
3. The main theorem/proof
4. Brief explanation of your approach"#,
            formalization.content, question.body
        );
        
        println!("🎓 {}:\n", prover.name);
        let output = prover.react_loop(&proof_prompt).await?;
        println!("{}\n", output.content.trim());
        prover_outputs.push(format!("[{}]\n{}", prover.name, output.content));
    }
    
    // Step 3: Debate rounds where provers critique each other
    for round in 1..=debate_rounds {
        println!("{}", "-".repeat(80));
        println!("⚔️  DEBATE ROUND {}\n", round);
        
        for (i, prover) in provers.iter().enumerate() {
            // Each prover critiques the previous prover's work
            let prev_idx = (i + provers.len() - 1) % provers.len();
            let prev_output = &prover_outputs[prev_idx];
            
            let critique_prompt = format!(
                r#"Review and critique this proof attempt, then provide an improved proof:

{}

Your original approach:
{}

Critique the other proof for:
1. Correctness issues
2. Missing cases
3. Style improvements
4. Better Lean4 idioms

Then provide your improved proof addressing any issues you found."#,
                prev_output, prover_outputs[i]
            );
            
            println!("🎓 {} (critique & improvement):\n", prover.name);
            let output = prover.react_loop(&critique_prompt).await?;
            println!("{}\n", output.content.trim());
            prover_outputs[i] = format!("[{}] REVISED\n{}", prover.name, output.content);
        }
    }
    
    // Step 4: Proctor synthesizes final proof
    println!("{}", "═".repeat(80));
    println!("🔮 SYNTHESIS\n");
    
    let all_proofs = prover_outputs.join("\n\n---\n\n");
    
    let synthesis_prompt = format!(
        r#"Synthesize the best proof from these theorem prover contributions:

## Original Question
{}

## Formalization
{}

## Prover Contributions
{}

## Your Task
1. Identify the strongest proof elements from each prover
2. Combine into a single, correct Lean4 proof
3. Provide an informal proof explanation
4. Summarize key insights from the debate

Format your response exactly as:

## Formalized Statement
<statement>

## Lean4 Proof
```lean4
<complete proof code>
```

## Informal Proof
<explanation>

## Debate Summary
<summary>"#,
        question.body, formalization.content, all_proofs
    );
    
    let synthesis = proctor.react_loop(&synthesis_prompt).await?;
    println!("📋 Final Synthesis:\n{}\n", synthesis.content);
    
    // Parse synthesis output
    let (formalized_statement, mut lean_proof, informal_proof, mut debate_summary) = 
        parse_synthesis(&synthesis.content);
    
    // Step 5: Verify the Lean4 proof
    let lean_available = check_lean_available();
    let mut verification_result = LeanVerificationResult {
        success: false,
        errors: None,
        output: String::new(),
    };
    let mut verification_attempts = 0u32;
    const MAX_VERIFICATION_ATTEMPTS: u32 = 3;
    
    if lean_available && !lean_proof.is_empty() {
        println!("{}", "═".repeat(80));
        println!("🔬 LEAN4 VERIFICATION\n");
        
        // Verification loop with retry
        while verification_attempts < MAX_VERIFICATION_ATTEMPTS {
            verification_attempts += 1;
            verification_result = verify_lean_proof(&lean_proof, &question.id);
            
            if verification_result.success {
                break;
            }
            
            // If failed and we have more attempts, run Consensus Fix
            if verification_attempts < MAX_VERIFICATION_ATTEMPTS {
                if let Some(ref errors) = verification_result.errors {
                    println!("\n🔄 Attempt {}/{}: Consensus Fix Round...\n", 
                             verification_attempts, MAX_VERIFICATION_ATTEMPTS);
                    println!("   Gathering fixes from all provers...");
                    
                    let fix_request = format!(
                        r#"The current Lean4 proof failed verification with these errors:
```
{}
```

Proof content:
```lean4
{}
```

Please fix the errors. 
CRITICAL SYNTAX CHECK:
- Ensure all parentheses `()` and braces `{{}}` are matched
- Check line endings and indentation
- Verify all definitions have type annotations
- NO Mathlib allowed (use `Std` or core only)

Provide the corrected Lean4 code."#, 
                        errors, lean_proof
                    );
                    
                    let mut fix_proposals = Vec::new();
                    for prover in provers {
                        print!("   ❓ Asking {}... ", prover.name);
                        // In a real implementation we would run these in parallel
                        // but sequential is fine for this example
                        match prover.react_loop(&fix_request).await {
                            Ok(resp) => {
                                println!("✓");
                                fix_proposals.push(format!("## {} Proposed Fix\n{}", prover.name, resp.content));
                            }
                            Err(e) => println!("✗ (Error: {})", e),
                        }
                    }
                    
                    println!("   🔮 Proctor synthesizing consensus fix...");
                    let consensus_prompt = format!(
                        r#"We are fixing verification errors in a Lean4 proof.
                        
Verification Errors:
```
{}
```

Proposed Fixes from Provers:
{}

Task:
1. Analyze the proposed fixes.
2. Synthesize a single, correct, standalone Lean4 proof (NO Mathlib).
3. Fix the specific errors reported.

Output ONLY the corrected Lean4 code in a code block."#,
                        errors, fix_proposals.join("\n\n")
                    );
                    
                    let fix_response = proctor.react_loop(&consensus_prompt).await?;
                    
                    // Extract the fixed proof from the response
                    let fixed_proof = extract_lean_code(&fix_response.content);
                    if !fixed_proof.is_empty() {
                        lean_proof = fixed_proof;
                        println!("   📝 Updated proof, re-verifying...\n");
                    }
                }
            }
        }
        
        if verification_result.success {
            println!("✅ Lean4 proof verified successfully after {} attempt(s)\n", verification_attempts);
            debate_summary = format!("{}\n\n**Lean4 Verification**: PASSED ({} attempts)", 
                                    debate_summary, verification_attempts);
        } else {
            println!("⚠️  Lean4 proof could not be verified after {} attempts\n", verification_attempts);
            debate_summary = format!("{}\n\n**Lean4 Verification**: FAILED ({} attempts)\nErrors: {}", 
                                    debate_summary, verification_attempts, 
                                    verification_result.errors.as_deref().unwrap_or("unknown"));
        }
    } else if !lean_available {
        println!("⚠️  Lean4 not available - skipping verification");
        println!("   Install with: curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh\n");
        debate_summary = format!("{}\n\n**Lean4 Verification**: SKIPPED (lean4 not installed)", debate_summary);
    }
    
    // Save the .lean file alongside the JSON
    let lean_dir = Path::new("data/mathoverflow/solved");
    if !lean_proof.is_empty() {
        let _ = save_lean_file(&question.id, &lean_proof, lean_dir);
    }
    
    let elapsed = start.elapsed();
    println!("⏱️  Completed in {:.1}s\n", elapsed.as_secs_f64());
    
    Ok(SolvedQuestion {
        id: question.id.clone(),
        mathoverflow_id: question.mathoverflow_id,
        url: question.url.clone(),
        title: question.title.clone(),
        original_question: question.body.clone(),
        formalized_statement,
        lean_proof,
        informal_proof,
        debate_summary,
        prover_contributions: ProverContributions {
            lean_formalist: prover_outputs.get(0).cloned().unwrap_or_default(),
            constructivist: prover_outputs.get(1).cloned().unwrap_or_default(),
            classical_reasoner: prover_outputs.get(2).cloned().unwrap_or_default(),
        },
        lean_verified: verification_result.success,
        lean_errors: verification_result.errors,
        verification_attempts,
        solved_at: Utc::now().to_rfc3339(),
    })
}

/// Extract Lean4 code from a response
fn extract_lean_code(content: &str) -> String {
    let mut in_code_block = false;
    let mut code = String::new();
    
    for line in content.lines() {
        if line.starts_with("```lean") {
            in_code_block = true;
            continue;
        } else if line == "```" && in_code_block {
            break;
        }
        
        if in_code_block {
            if !code.is_empty() {
                code.push('\n');
            }
            code.push_str(line);
        }
    }
    
    code
}

/// Parse synthesis output into components
fn parse_synthesis(content: &str) -> (String, String, String, String) {
    let mut formalized = String::new();
    let mut lean_proof = String::new();
    let mut informal = String::new();
    let mut summary = String::new();
    
    let mut current_section = "";
    let mut in_code_block = false;
    
    for line in content.lines() {
        if line.starts_with("## Formalized Statement") {
            current_section = "formalized";
            continue;
        } else if line.starts_with("## Lean4 Proof") || line.starts_with("## Lean Proof") {
            current_section = "lean";
            continue;
        } else if line.starts_with("## Informal Proof") {
            current_section = "informal";
            continue;
        } else if line.starts_with("## Debate Summary") {
            current_section = "summary";
            continue;
        }
        
        if line.starts_with("```lean") {
            in_code_block = true;
            continue;
        } else if line == "```" && in_code_block {
            in_code_block = false;
            continue;
        }
        
        let target = match current_section {
            "formalized" => &mut formalized,
            "lean" => &mut lean_proof,
            "informal" => &mut informal,
            "summary" => &mut summary,
            _ => continue,
        };
        
        if !target.is_empty() {
            target.push('\n');
        }
        target.push_str(line);
    }
    
    (
        formalized.trim().to_string(),
        lean_proof.trim().to_string(),
        informal.trim().to_string(),
        summary.trim().to_string(),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("\n{}", "═".repeat(80));
    println!("📐 MATHOVERFLOW THEOREM PROVER");
    println!("   Multi-Agent Debate-Based Proof System with Lean4");
    println!("{}\n", "═".repeat(80));
    
    // Parse command line args
    let args: Vec<String> = std::env::args().collect();
    let limit = args.iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    
    let debug = args.iter().any(|a| a == "--debug");
    let debate_rounds = args.iter()
        .position(|a| a == "--rounds")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    
    // Initialize OpenRouter client
    let client: Arc<dyn LlmClient> = match OpenRouterClient::from_env() {
        Ok(c) => {
            println!("✓ OpenRouter client ready\n");
            Arc::new(c)
        }
        Err(e) => {
            eprintln!("✗ OpenRouter not available: {}", e);
            eprintln!("  Set OPENROUTER_API_KEY in .env to run this example");
            return Ok(());
        }
    };
    
    // Load questions
    let questions_dir = Path::new("data/mathoverflow/questions");
    let questions = load_questions(questions_dir)?;
    
    if questions.is_empty() {
        println!("❌ No questions found in {:?}", questions_dir);
        println!("   Run: ./tools/mathoverflow_scraper --limit 5");
        return Ok(());
    }
    
    let questions_to_process: Vec<_> = questions.into_iter().take(limit).collect();
    
    println!("📚 Loaded {} questions to prove\n", questions_to_process.len());
    for (i, q) in questions_to_process.iter().enumerate() {
        println!("   {}. [{}] {}", i + 1, q.id, q.title);
    }
    
    // Build agents
    println!("\n🤖 Building theorem prover agents...");
    
    let provers = vec![
        build_prover_agent(ProverStyle::LeanFormalist, client.clone())?,
        build_prover_agent(ProverStyle::Constructivist, client.clone())?,
        build_prover_agent(ProverStyle::ClassicalReasoner, client.clone())?,
    ];
    
    for prover in &provers {
        println!("   ✓ {}", prover.name);
    }
    
    let proctor = build_proctor_agent(client.clone())?;
    println!("   ✓ {} (Proctor)\n", proctor.name);
    
    println!("⚙️  Configuration:");
    println!("   • Debate rounds: {}", debate_rounds);
    println!("   • Debug mode: {}", debug);
    
    // Process each question
    let solved_dir = Path::new("data/mathoverflow/solved");
    let mut solved_count = 0;
    let mut failed_count = 0;
    
    for question in &questions_to_process {
        match prove_question(question, &provers, &proctor, debate_rounds).await {
            Ok(solved) => {
                save_solved(&solved, solved_dir)?;
                if solved.lean_verified {
                    println!("✅ Saved (Verified): {}_solved.json\n", solved.id);
                    solved_count += 1;
                } else {
                    println!("⚠️  Saved (Unverified): {}_solved.json (Verification Failed)\n", solved.id);
                    failed_count += 1;
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to prove {}: {}\n", question.id, e);
                failed_count += 1;
                
                // Save partial progress
                let partial = SolvedQuestion {
                    id: question.id.clone(),
                    mathoverflow_id: question.mathoverflow_id,
                    url: question.url.clone(),
                    title: question.title.clone(),
                    original_question: question.body.clone(),
                    formalized_statement: String::new(),
                    lean_proof: format!("-- ERROR: {}", e),
                    informal_proof: String::new(),
                    debate_summary: format!("Failed with error: {}", e),
                    prover_contributions: ProverContributions {
                        lean_formalist: String::new(),
                        constructivist: String::new(),
                        classical_reasoner: String::new(),
                    },
                    lean_verified: false,
                    lean_errors: Some(e.to_string()),
                    verification_attempts: 0,
                    solved_at: Utc::now().to_rfc3339(),
                };
                save_solved(&partial, solved_dir)?;
            }
        }
    }
    
    // Summary
    println!("\n{}", "═".repeat(80));
    println!("✨ THEOREM PROVING COMPLETE");
    println!("{}\n", "═".repeat(80));
    
    println!("📊 Results:");
    println!("   • Questions processed: {}", questions_to_process.len());
    println!("   • Successfully proved: {}", solved_count);
    println!("   • Failed: {}", failed_count);
    println!("   • Output directory: {:?}", solved_dir);
    
    println!("\n🎓 Theorem Provers:");
    for prover in &provers {
        println!("   • {}", prover.name);
    }
    
    println!("\n📁 View results:");
    println!("   ls data/mathoverflow/solved/");
    println!("   cat data/mathoverflow/solved/*_solved.json | jq '.lean_proof'\n");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_styles_defined() {
        for style in [ProverStyle::LeanFormalist, ProverStyle::Constructivist, ProverStyle::ClassicalReasoner] {
            assert!(!style.name().is_empty());
            assert!(!style.system_prompt().is_empty());
            assert!(style.system_prompt().contains("Lean"));
        }
    }

    #[test]
    fn test_parse_synthesis() {
        let content = r#"## Formalized Statement
For all n : Nat, n + 0 = n

## Lean4 Proof
```lean4
theorem nat_add_zero (n : Nat) : n + 0 = n := Nat.add_zero n
```

## Informal Proof
By definition of addition on natural numbers.

## Debate Summary
All provers agreed on this basic property."#;

        let (formalized, lean, informal, summary) = parse_synthesis(content);
        
        assert!(formalized.contains("For all n"));
        assert!(lean.contains("theorem nat_add_zero"));
        assert!(informal.contains("definition"));
        assert!(summary.contains("agreed"));
    }

    #[test]
    fn test_question_serde() {
        let q = ScrapedQuestion {
            id: "q_12345".to_string(),
            mathoverflow_id: 12345,
            url: "https://mathoverflow.net/questions/12345".to_string(),
            title: "Test Question".to_string(),
            body: "Is P = NP?".to_string(),
            tags: "complexity-theory".to_string(),
            scraped_at: "2026-01-14T00:00:00Z".to_string(),
            answer: "".to_string(),
        };
        
        let json = serde_json::to_string(&q).unwrap();
        let parsed: ScrapedQuestion = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.id, q.id);
        assert_eq!(parsed.mathoverflow_id, q.mathoverflow_id);
    }
}
