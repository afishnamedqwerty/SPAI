//! Monetary Policy Debate: Keynesian vs Hayek Economists
//!
//! This example demonstrates a multi-agent debate between economic schools of thought:
//! - 3 Keynesian economists (pro government intervention, fiscal stimulus)
//! - 3 Hayek/Austrian economists (pro market self-correction, minimal intervention)
//!
//! Debate Topics:
//! 1. Assessment of current Federal Reserve policy decisions
//! 2. Probability distribution of policy challenges (AI disruption, geopolitical tensions)
//! 3. Solutions for sovereign debt without excessive taxes, seizures, or inflation
//!
//! Uses the debate workflow pattern with team-based argumentation.

use spai::prelude::*;
use spai::orchestrator::{
    OrchestratorPattern,
    DebateOrchestrator,
};
use std::sync::Arc;
use std::time::Instant;

/// Debate topics for the monetary policy discussion
const TOPIC_1_FED_POLICY: &str = r#"
Assess the current Federal Reserve monetary policy decisions as of January 2026.

Consider:
- The current interest rate trajectory and its effects
- Quantitative easing/tightening policies and balance sheet management
- The Fed's dual mandate (price stability and maximum employment)
- Recent policy pivots and their market implications
- The sustainability of current policy stances

Provide concrete analysis with supporting economic reasoning.
"#;

const TOPIC_2_DISRUPTION_RISKS: &str = r#"
Analyze the probability distribution of challenges to established monetary policy due to:

1. AI/Automation Disruption:
   - Labor market transformations affecting employment mandate
   - Productivity gains vs deflationary pressures
   - Central bank modeling failures in new paradigms

2. Geopolitical Tensions:
   - De-dollarization efforts and reserve currency status
   - Supply chain reshoring and inflationary pressures
   - Sanctions regimes affecting financial system stability
   - Energy market volatility and stagflation risks

3. Structural Economic Shifts:
   - Aging demographics and entitlement pressures
   - Climate transition costs
   - Cryptocurrency/CBDC competition

Estimate likelihood, timing, and severity of each risk category.
"#;

const TOPIC_3_DEBT_SOLUTIONS: &str = r#"
Propose solutions to tackle the risks of unpayable sovereign debt WITHOUT resulting in:
- Exorbitant taxes (that would destroy economic growth)
- Asset seizures (that would undermine property rights)
- Inflation to zero purchasing power (that would devastate savers)

Current Context:
- US federal debt exceeds $35 trillion (approximately 125% of GDP)
- Interest payments now exceed $1 trillion annually
- Entitlement spending on autopilot trajectory
- Traditional policy options appear exhausted

Requirements:
1. Solutions must be economically viable, not politically naive
2. Address both short-term stability and long-term sustainability
3. Consider international coordination requirements
4. Evaluate trade-offs honestly and completely

Provide a concrete policy framework with implementation pathway.
"#;

/// Creates a Keynesian economist persona
fn create_keynesian_persona(index: usize) -> (String, String) {
    let personas = [
        (
            "Dr. Janet Wiseman (Neo-Keynesian)",
            r#"You are Dr. Janet Wiseman, a prominent Neo-Keynesian economist and former Federal Reserve advisor.

Your Economic Philosophy:
- Markets are prone to failures and require active government intervention
- Aggregate demand drives economic output; supply follows demand
- Sticky wages and prices prevent rapid market clearing
- The government has tools (fiscal and monetary policy) to stabilize business cycles
- In recessions, deficit spending is not just acceptable but necessary
- The multiplier effect amplifies government spending

Key Positions:
- Support aggressive Fed intervention during crises
- Favor expansionary policy when unemployment is high
- Believe in Phillips Curve trade-offs (inflation vs unemployment)
- Accept Modern Monetary Theory insights on sovereign currency issuance
- Trust technocratic management of the economy

Debate Style:
- Reference empirical data, especially from 2008 crisis and COVID response
- Cite Keynes, Samuelson, Krugman, Stiglitz
- Emphasize social costs of unemployment and inequality
- Challenge Austrian assumptions about market self-correction"#,
        ),
        (
            "Prof. Robert Summers (Post-Keynesian)",
            r#"You are Professor Robert Summers, a Post-Keynesian economist specializing in financial instability.

Your Economic Philosophy:
- Financial markets are inherently unstable (Minsky's Financial Instability Hypothesis)
- Uncertainty (Knightian) makes rational expectations unrealistic
- Endogenous money creation by banks drives credit cycles
- Stock-flow consistent modeling reveals balance sheet dependencies
- Distribution of income/wealth affects aggregate demand critically

Key Positions:
- Banks create money through lending; Fed controls credit conditions, not money supply
- Asset price bubbles require proactive regulatory intervention
- Austerity during downturns is self-defeating (fiscal contraction paradox)
- Support financial repression to manage debt ratios
- Favor incomes policies alongside monetary policy
- Skeptical of inflation targeting as sole mandate

Debate Style:
- Reference Minsky, Godley, Kalecki, Joan Robinson
- Emphasize endogeneity of money and credit cycles
- Challenge quantity theory of money assumptions
- Use sectoral balance analysis
- Highlight role of expectations and conventions"#,
        ),
        (
            "Dr. Maria Chen (New Keynesian DSGE)",
            r#"You are Dr. Maria Chen, a leading New Keynesian macroeconomist who develops DSGE models at a major central bank research department.

Your Economic Philosophy:
- Nominal rigidities (sticky prices, wages) justify monetary policy activism
- Rational expectations with microfoundations for macro models
- Taylor Rules provide good guidance for interest rate policy
- Forward guidance can substitute for policy rate changes at zero lower bound
- Unconventional monetary policies (QE, yield curve control) are effective

Key Positions:
- Support flexible inflation targeting (2% symmetric)
- Central bank independence is sacrosanct
- Fiscal policy has a role but must consider Ricardian equivalence effects
- Labor market flexibility reduces policy trade-offs
- Optimal policy responds to output gaps and inflation deviations
- DSGE models provide rigorous policy evaluation framework

Debate Style:
- Reference Woodford, Clarida, Galí, Gertler
- Use dynamic general equilibrium frameworks
- Emphasize intertemporal optimization and expectations channels
- Quantitative modeling and empirical identification strategies
- Acknowledge model limitations while defending core insights"#,
        ),
    ];
    
    let (name, prompt) = &personas[index];
    (name.to_string(), prompt.to_string())
}

/// Creates a Hayek/Austrian economist persona
fn create_hayek_persona(index: usize) -> (String, String) {
    let personas = [
        (
            "Dr. Friedrich Richter (Austrian School)",
            r#"You are Dr. Friedrich Richter, a leading Austrian School economist in the tradition of Mises and Hayek.

Your Economic Philosophy:
- Markets coordinate decentralized knowledge that no central planner can possess
- Price signals are essential information carriers; distorting them causes malinvestment
- Business cycles are caused by credit expansion beyond real savings (Austrian Business Cycle Theory)
- Sustainable growth requires real savings, not credit creation
- Time preference determines interest rates naturally
- Entrepreneurship drives discovery and innovation in unplanned ways

Key Positions:
- Federal Reserve creates boom-bust cycles through artificial credit expansion
- Quantitative easing distorts asset prices and misallocates capital
- Low interest rates cause malinvestment that must eventually correct
- Oppose bailouts; creative destruction is necessary for renewal
- Sound money (or at least rule-based policy) is essential
- Central banking itself is the problem, not the solution

Debate Style:
- Reference Mises, Hayek, Rothbard, Garrison
- Emphasize coordination problems and dispersed knowledge
- Challenge macroeconomic aggregates as meaningful
- Use Austrian Business Cycle Theory framework
- Highlight unintended consequences of intervention
- Advocate for market discovery processes"#,
        ),
        (
            "Prof. James Buchanan III (Public Choice)",
            r#"You are Professor James Buchanan III, a Public Choice economist who applies rational choice theory to political decision-making.

Your Economic Philosophy:
- Politicians and bureaucrats are self-interested, not benevolent social planners
- Constitutional constraints on government are essential for liberty
- Concentrated benefits and dispersed costs drive bad policy
- Debt allows politicians to spend without immediate tax pain
- Central bank independence is politically impossible over long horizons
- Regulatory capture ensures agencies serve special interests

Key Positions:
- Fiscal discipline requires constitutional debt brakes
- The Fed is subject to political pressure despite nominal independence
- Entitlement spending ratchets are nearly impossible to reverse democratically
- Inflationary bias results from political pressure on central banks
- Sound money constitutions (gold standard, Bitcoin) constrain political abuse
- Decentralization limits governmental overreach

Debate Style:
- Reference Buchanan (Nobel 1986), Tullock, Niskanen, Olson
- Apply economic analysis to political institutions
- Emphasize incentive structures and institutional design
- Challenge "market failure → government intervention" logic
- Highlight government failure as worse than market failure
- Advocate constitutional constraints on policy"#,
        ),
        (
            "Dr. Sarah Thornton (Free Banking / Selgin School)",
            r#"You are Dr. Sarah Thornton, a free banking economist specializing in monetary theory and history.

Your Economic Philosophy:
- Competitive issue of money by private banks historically outperformed central banking
- Central bank monopoly on money creation removes competitive discipline
- Free banking systems (Scotland, Canada pre-1935) showed remarkable stability
- NGDP targeting is second-best if central banking continues
- Cryptocurrency and decentralized finance offer new forms of monetary competition
- Base money should grow at predictable rates, not discretionary policy

Key Positions:
- The Fed's dual mandate is inherently conflicted and discretion-prone
- Historical evidence shows free banking was not chaotic but self-regulating
- Central banks systematically inflate beyond price stability
- Crisis interventions create moral hazard encouraging future risk-taking
- NGDP level targeting would reduce discretionary policy errors
- Competition in money issuance (including stablecoins, Bitcoin) should be permitted

Debate Style:
- Reference Selgin, White, Sargent, Wallace, Hayek on denationalization
- Use historical evidence from free banking episodes
- Emphasize monetary rules over discretion
- Apply game theory to monetary institutions
- Highlight quantity theory insights but avoid crude monetarism
- Propose practical reforms toward more rule-based policy"#,
        ),
    ];
    
    let (name, prompt) = &personas[index];
    (name.to_string(), prompt.to_string())
}

/// Build an economist agent
fn build_economist_agent(
    name: &str,
    system_prompt: &str,
    client: Arc<dyn LlmClient>,
) -> anyhow::Result<Agent> {
    Agent::builder()
        .name(name)
        .model("tngtech/deepseek-r1t2-chimera:free")
        .system_prompt(system_prompt)
        .max_loops(3)
        .temperature(0.8)
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

/// Build synthesizer agent
fn build_synthesizer(client: Arc<dyn LlmClient>) -> anyhow::Result<Agent> {
    Agent::builder()
        .name("Dr. Neutral Arbiter (Economic Policy Synthesizer)")
        .model("anthropic/claude-sonnet-4")
        .system_prompt(r#"You are Dr. Neutral Arbiter, a balanced economic policy analyst tasked with synthesizing opposing viewpoints.

Your Role:
1. Identify the strongest arguments from both Keynesian and Austrian/Hayek perspectives
2. Find areas of genuine agreement (they exist!)
3. Weigh empirical evidence fairly
4. Acknowledge uncertainty honestly
5. Produce actionable, balanced policy recommendations

Your Standards:
- Steel-man both positions before critiquing
- Cite specific evidence for claims
- Acknowledge trade-offs explicitly
- Avoid false equivalence - some arguments ARE stronger
- Consider political feasibility alongside economic soundness
- Synthesize, don't just summarize

Output Format:
Provide a structured synthesis with:
- Key agreements between schools
- Fundamental disagreements and their sources
- Strongest arguments from each side
- Your reasoned assessment
- Concrete policy recommendations with confidence levels"#)
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

/// Run a multi-team debate with Keynesian vs Hayek economists
async fn run_team_debate(
    topic: &str,
    topic_name: &str,
    keynesian_team: &[Agent],
    hayek_team: &[Agent],
    synthesizer: &Agent,
    debate_rounds: usize,
) -> anyhow::Result<String> {
    let start = Instant::now();
    
    println!("\n{}", "═".repeat(80));
    println!("📊 DEBATE TOPIC: {}", topic_name);
    println!("{}\n", "═".repeat(80));
    
    let mut keynesian_arguments: Vec<String> = Vec::new();
    let mut hayek_arguments: Vec<String> = Vec::new();
    
    // Initial statements from each team
    println!("🔹 OPENING STATEMENTS\n");
    
    // Keynesian team opening
    println!("📘 KEYNESIAN TEAM OPENING:\n");
    for (i, agent) in keynesian_team.iter().enumerate() {
        let prompt = format!(
            "Topic: {}\n\nAs a Keynesian economist, provide your opening analysis and arguments. \
             Focus on your area of expertise. Be specific and cite economic theory/evidence.",
            topic
        );
        
        let output = agent.react_loop(&prompt).await?;
        keynesian_arguments.push(format!("[{}] {}", agent.name, output.content.clone()));
        
        println!("  🎓 {}:\n", agent.name);
        println!("  {}\n", output.content.trim().replace("\n", "\n  "));
    }
    
    // Hayek team opening
    println!("\n📕 HAYEK/AUSTRIAN TEAM OPENING:\n");
    for (i, agent) in hayek_team.iter().enumerate() {
        let keynesian_summary = keynesian_arguments.join("\n\n");
        let prompt = format!(
            "Topic: {}\n\nThe Keynesian team has argued:\n{}\n\n\
             As an Austrian/Hayek-tradition economist, provide your counter-analysis and arguments. \
             Respond to their points while presenting your own framework.",
            topic, keynesian_summary
        );
        
        let output = agent.react_loop(&prompt).await?;
        hayek_arguments.push(format!("[{}] {}", agent.name, output.content.clone()));
        
        println!("  🎓 {}:\n", agent.name);
        println!("  {}\n", output.content.trim().replace("\n", "\n  "));
    }
    
    // Additional debate rounds (rebuttals)
    for round in 1..=debate_rounds {
        println!("\n{}", "-".repeat(80));
        println!("⚔️  REBUTTAL ROUND {}", round);
        println!("{}\n", "-".repeat(80));
        
        // Keynesian rebuttal
        println!("📘 KEYNESIAN REBUTTAL:\n");
        let hayek_summary = hayek_arguments.last().cloned().unwrap_or_default();
        
        // Pick one agent to rebut (rotating)
        let rebutter = &keynesian_team[round % keynesian_team.len()];
        let prompt = format!(
            "The Austrian/Hayek economists have argued:\n{}\n\n\
             Provide a focused rebuttal to their strongest points. \
             Defend the Keynesian position with evidence and counter-arguments.",
            hayek_summary
        );
        
        let output = rebutter.react_loop(&prompt).await?;
        keynesian_arguments.push(format!("[{}] REBUTTAL: {}", rebutter.name, output.content.clone()));
        
        println!("  🎓 {} (Rebuttal):\n", rebutter.name);
        println!("  {}\n", output.content.trim().replace("\n", "\n  "));
        
        // Hayek rebuttal
        println!("\n📕 HAYEK/AUSTRIAN REBUTTAL:\n");
        let keynesian_summary = keynesian_arguments.last().cloned().unwrap_or_default();
        
        let rebutter = &hayek_team[round % hayek_team.len()];
        let prompt = format!(
            "The Keynesian economists have argued:\n{}\n\n\
             Provide a focused rebuttal to their strongest points. \
             Defend the Austrian/Hayek position with evidence and counter-arguments.",
            keynesian_summary
        );
        
        let output = rebutter.react_loop(&prompt).await?;
        hayek_arguments.push(format!("[{}] REBUTTAL: {}", rebutter.name, output.content.clone()));
        
        println!("  🎓 {} (Rebuttal):\n", rebutter.name);
        println!("  {}\n", output.content.trim().replace("\n", "\n  "));
    }
    
    // Synthesis
    println!("\n{}", "═".repeat(80));
    println!("🔮 SYNTHESIS");
    println!("{}\n", "═".repeat(80));
    
    let full_keynesian = keynesian_arguments.join("\n\n");
    let full_hayek = hayek_arguments.join("\n\n");
    
    let synthesis_prompt = format!(
        r#"# Economic Debate: {}

## KEYNESIAN ARGUMENTS:
{}

## HAYEK/AUSTRIAN ARGUMENTS:
{}

## YOUR TASK:
Synthesize this debate into a balanced, actionable analysis. Identify:
1. Where each side makes valid points
2. Where each side has blind spots
3. What empirical evidence supports each position
4. Your reasoned policy recommendations
5. Confidence levels and key uncertainties

Be thorough but action-oriented. This is for policymakers, not academics."#,
        topic_name, full_keynesian, full_hayek
    );
    
    let synthesis = synthesizer.react_loop(&synthesis_prompt).await?;
    
    println!("🏛️  Dr. Neutral Arbiter's Synthesis:\n");
    println!("{}\n", synthesis.content);
    
    let elapsed = start.elapsed();
    println!("⏱️  Debate completed in {:.1}s\n", elapsed.as_secs_f64());
    
    Ok(synthesis.content)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("\n{}", "═".repeat(80));
    println!("💰 MONETARY POLICY DEBATE: KEYNESIAN vs HAYEK");
    println!("   Multi-Agent Economic Analysis Framework");
    println!("{}\n", "═".repeat(80));
    
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
    
    // Build Keynesian team (3 economists)
    println!("🔵 Building Keynesian Economics Team...");
    let mut keynesian_team = Vec::new();
    for i in 0..3 {
        let (name, prompt) = create_keynesian_persona(i);
        let agent = build_economist_agent(&name, &prompt, client.clone())?;
        println!("   ✓ {}", name);
        keynesian_team.push(agent);
    }
    
    // Build Hayek team (3 economists)
    println!("\n🔴 Building Hayek/Austrian Economics Team...");
    let mut hayek_team = Vec::new();
    for i in 0..3 {
        let (name, prompt) = create_hayek_persona(i);
        let agent = build_economist_agent(&name, &prompt, client.clone())?;
        println!("   ✓ {}", name);
        hayek_team.push(agent);
    }
    
    // Build synthesizer
    println!("\n⚖️  Building Neutral Synthesizer...");
    let synthesizer = build_synthesizer(client.clone())?;
    println!("   ✓ Dr. Neutral Arbiter\n");
    
    println!("{}", "=".repeat(80));
    println!("🎯 DEBATE AGENDA:");
    println!("   1. Federal Reserve Policy Assessment (January 2026)");
    println!("   2. Risk Probability Distribution (AI, Geopolitical, Structural)");
    println!("   3. Sovereign Debt Solutions (No Excessive Taxes/Seizures/Inflation)");
    println!("{}\n", "=".repeat(80));
    
    // Store all syntheses for final summary
    let mut all_syntheses = Vec::new();
    
    // ═══════════════════════════════════════════════════════════════════════════
    // TOPIC 1: Federal Reserve Policy Assessment
    // ═══════════════════════════════════════════════════════════════════════════
    
    let synthesis_1 = run_team_debate(
        TOPIC_1_FED_POLICY,
        "Federal Reserve Policy Assessment (January 2026)",
        &keynesian_team,
        &hayek_team,
        &synthesizer,
        1, // 1 rebuttal round
    ).await?;
    all_syntheses.push(("Federal Reserve Policy", synthesis_1));
    
    // ═══════════════════════════════════════════════════════════════════════════
    // TOPIC 2: Risk Probability Distribution
    // ═══════════════════════════════════════════════════════════════════════════
    
    let synthesis_2 = run_team_debate(
        TOPIC_2_DISRUPTION_RISKS,
        "Policy Challenges: AI Disruption & Geopolitical Risks",
        &keynesian_team,
        &hayek_team,
        &synthesizer,
        1, // 1 rebuttal round
    ).await?;
    all_syntheses.push(("Disruption Risks", synthesis_2));
    
    // ═══════════════════════════════════════════════════════════════════════════
    // TOPIC 3: Sovereign Debt Solutions
    // ═══════════════════════════════════════════════════════════════════════════
    
    let synthesis_3 = run_team_debate(
        TOPIC_3_DEBT_SOLUTIONS,
        "Sovereign Debt Crisis: Solutions Without Destruction",
        &keynesian_team,
        &hayek_team,
        &synthesizer,
        2, // 2 rebuttal rounds for this critical topic
    ).await?;
    all_syntheses.push(("Debt Solutions", synthesis_3));
    
    // ═══════════════════════════════════════════════════════════════════════════
    // FINAL INTEGRATED POLICY FRAMEWORK
    // ═══════════════════════════════════════════════════════════════════════════
    
    println!("\n{}", "═".repeat(80));
    println!("🏛️  FINAL INTEGRATED POLICY FRAMEWORK");
    println!("{}\n", "═".repeat(80));
    
    let integrated_prompt = format!(
        r#"You have synthesized three debates between Keynesian and Austrian/Hayek economists:

## Debate 1: Federal Reserve Policy
{}

## Debate 2: Disruption Risks
{}

## Debate 3: Sovereign Debt Solutions
{}

## YOUR FINAL TASK:
Produce an INTEGRATED POLICY FRAMEWORK that:

1. **Coherent Strategy**: Ensures the three sets of recommendations work together
2. **Prioritized Actions**: Rank recommendations by urgency and impact
3. **Implementation Pathway**: Realistic sequencing of policy changes
4. **Political Feasibility**: Acknowledge constraints and coalition requirements
5. **Risk Management**: How to adapt if key assumptions prove wrong
6. **Success Metrics**: How we know if policies are working

This should be a practical policy memo suitable for the Federal Reserve Chair and Treasury Secretary."#,
        all_syntheses[0].1,
        all_syntheses[1].1,
        all_syntheses[2].1
    );
    
    let final_framework = synthesizer.react_loop(&integrated_prompt).await?;
    
    println!("📋 INTEGRATED POLICY RECOMMENDATION:\n");
    println!("{}\n", final_framework.content);
    
    // ═══════════════════════════════════════════════════════════════════════════
    // COMPLETION SUMMARY
    // ═══════════════════════════════════════════════════════════════════════════
    
    println!("\n{}", "═".repeat(80));
    println!("✨ MONETARY POLICY DEBATE COMPLETE");
    println!("{}\n", "═".repeat(80));
    
    println!("📊 Debate Statistics:");
    println!("   • Keynesian economists: 3");
    println!("   • Hayek/Austrian economists: 3");
    println!("   • Topics debated: 3");
    println!("   • Total debate rounds: 7+ exchanges");
    println!("   • Synthesis reports: 4 (3 topic + 1 integrated)");
    
    println!("\n🎓 Participants:");
    for agent in &keynesian_team {
        println!("   📘 {}", agent.name);
    }
    for agent in &hayek_team {
        println!("   📕 {}", agent.name);
    }
    println!("   ⚖️  {}", synthesizer.name);
    
    println!("\n💡 Key Insights Available:");
    println!("   1. Fed Policy Assessment with bipartisan economic analysis");
    println!("   2. Risk probability estimates from opposing theoretical frameworks");
    println!("   3. Debt solutions that avoid confiscatory outcomes");
    println!("   4. Integrated policy framework bridging economic schools");
    
    println!("\n📁 To save results, redirect output: cargo run --example monetary_policy > debate_results.md\n");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keynesian_personas_created() {
        for i in 0..3 {
            let (name, prompt) = create_keynesian_persona(i);
            assert!(!name.is_empty());
            assert!(!prompt.is_empty());
            assert!(prompt.contains("Keynesian") || prompt.contains("demand") || prompt.contains("fiscal"));
        }
    }

    #[test]
    fn test_hayek_personas_created() {
        for i in 0..3 {
            let (name, prompt) = create_hayek_persona(i);
            assert!(!name.is_empty());
            assert!(!prompt.is_empty());
            assert!(
                prompt.contains("Austrian") || 
                prompt.contains("Hayek") || 
                prompt.contains("market") ||
                prompt.contains("Public Choice")
            );
        }
    }

    #[test]
    fn test_topic_definitions_present() {
        assert!(!TOPIC_1_FED_POLICY.is_empty());
        assert!(!TOPIC_2_DISRUPTION_RISKS.is_empty());
        assert!(!TOPIC_3_DEBT_SOLUTIONS.is_empty());
        
        // Verify key content
        assert!(TOPIC_1_FED_POLICY.contains("Federal Reserve"));
        assert!(TOPIC_2_DISRUPTION_RISKS.contains("AI"));
        assert!(TOPIC_2_DISRUPTION_RISKS.contains("geopolitical"));
        assert!(TOPIC_3_DEBT_SOLUTIONS.contains("sovereign debt"));
        assert!(TOPIC_3_DEBT_SOLUTIONS.contains("inflation"));
    }
}
