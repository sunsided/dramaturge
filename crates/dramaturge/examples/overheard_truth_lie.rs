//! # The Broken Seal: A Dramaturge Dialogue Example (YAML-Loaded)
//!
//! This example demonstrates loading dialogue configuration from YAML files.
//! The runtime engine loads the dialogue graph at startup and executes it.
//!
//! ## Features Demonstrated
//!
//! - Loading dialogue graphs from YAML configuration
//! - Runtime validation of conditions and outcomes
//! - Interactive TUI with debug mode
//!
//! ## Run the Example
//!
//! ```bash
//! cargo run --example overheard_truth_lie
//! ```

use dramaturge::*;
use dramaturge_serde::loader::YamlDialogueGraph;
use std::collections::HashMap;
use std::io::{self, Write};

// =============================================================================
// CHARACTER IDS
// =============================================================================

mod entities {
    use dramaturge::EntityId;

    pub const PLAYER: u64 = 1;
    pub const MARA: u64 = 2;
    pub const ORREN: u64 = 3;

    pub fn player() -> EntityId {
        EntityId::new(PLAYER)
    }
    pub fn mara() -> EntityId {
        EntityId::new(MARA)
    }
    pub fn orren() -> EntityId {
        EntityId::new(ORREN)
    }
}

// =============================================================================
// GAME STATE
// =============================================================================

struct GameState {
    world: DialogWorldState,
    current_node: DialogueNodeId,
    debug_mode: bool,
    graph: DialogueGraph,
    character_names: HashMap<u64, &'static str>,
}

impl GameState {
    fn new(graph: DialogueGraph) -> Self {
        let mut world = DialogWorldState::new();

        // Initialize player skills
        world.add_skill("medic");
        world.add_skill("investigator");
        world.add_skill("streetwise");
        // Note: player does NOT have intimidation skill - that option should be unavailable

        // Initialize relationships
        world.add_relationship(
            entities::player(),
            entities::mara(),
            RelationshipAxisId::new("trust"),
            0,
        );
        world.add_relationship(
            entities::player(),
            entities::orren(),
            RelationshipAxisId::new("trust"),
            0,
        );
        world.add_relationship(
            entities::player(),
            entities::orren(),
            RelationshipAxisId::new("standing"),
            0,
        );
        world.add_relationship(
            entities::player(),
            entities::mara(),
            RelationshipAxisId::new("standing"),
            0,
        );

        let character_names: HashMap<u64, &'static str> = [
            (entities::PLAYER, "Player"),
            (entities::MARA, "Mara Vey"),
            (entities::ORREN, "Captain Orren Vale"),
        ]
        .into_iter()
        .collect();

        let entry_node = graph.entry_node.unwrap_or(DialogueNodeId::new(1));

        Self {
            world,
            current_node: entry_node,
            debug_mode: false,
            graph,
            character_names,
        }
    }

    fn current_node(&self) -> Option<&DialogueNode> {
        self.graph.nodes.iter().find(|n| n.id == self.current_node)
    }

    fn available_options(&self) -> Vec<&DialogueOption> {
        let evaluator = DefaultConditionEvaluator;
        let node = match self.current_node() {
            Some(n) => n,
            None => return vec![],
        };

        node.options
            .iter()
            .filter(|opt| {
                // Check explicit conditions
                let conditions_pass = opt
                    .conditions
                    .iter()
                    .all(|c| evaluator.evaluate(c, &self.world));

                // Check skill requirements from tags
                let skill_requirements_pass = opt.tags.iter().all(|tag| {
                    if tag.requires_trait {
                        // Check if player has the required skill
                        self.world.has_skill(&tag.key)
                    } else {
                        true
                    }
                });

                conditions_pass && skill_requirements_pass
            })
            .collect()
    }

    fn get_speaker_name(&self, entity_id: EntityId) -> &'static str {
        self.character_names
            .get(&entity_id.0)
            .copied()
            .unwrap_or("Unknown")
    }

    fn apply_option(&mut self, option_index: usize) {
        let option_data = {
            let options = self.available_options();
            if option_index >= options.len() {
                return;
            }
            let option = options[option_index];
            (
                option.id,
                option.tags.clone(),
                option.intentions.clone(),
                option.outcomes.clone(),
                option.target.clone(),
            )
        };

        let (option_id, tags, intentions, outcomes, target) = option_data;
        let mut applier = DefaultOutcomeApplier;

        for outcome in &outcomes {
            applier.apply(outcome, &mut self.world);
        }

        let event_kind = EventKind::DialogOptionChosen {
            node_id: self.current_node,
            option_id,
            tags,
            intentions,
        };
        self.world.emit_event(event_kind);

        // Hidden observation logic
        self.handle_hidden_observation();

        match &target {
            DialogTarget::Node(node_id) => {
                self.current_node = *node_id;
                // Apply node outcomes
                if let Some(node) = self.graph.nodes.iter().find(|n| n.id == *node_id) {
                    for outcome in &node.outcomes_on_enter {
                        applier.apply(outcome, &mut self.world);
                    }
                }
            }
            DialogTarget::End => {
                self.show_ending();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    fn handle_hidden_observation(&mut self) {
        // Orren overhears Mara conversations (nodes 1-9)
        if self.current_node.0 >= 1 && self.current_node.0 <= 9 {
            let mut applier = DefaultOutcomeApplier;
            applier.apply(
                &set_hidden_fact("OrrenOverheardMaraConversation"),
                &mut self.world,
            );
        }
    }

    fn show_ending(&self) {
        clear_screen();
        println!("╔══════════════════════════════════════════════════════════════════╗");
        println!("║                      THE BROKEN SEAL - END                       ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!();

        let mara_trust = self
            .world
            .get_relationship(
                entities::player(),
                entities::mara(),
                &RelationshipAxisId::new("trust"),
            )
            .unwrap_or(0);
        let orren_trust = self
            .world
            .get_relationship(
                entities::player(),
                entities::orren(),
                &RelationshipAxisId::new("trust"),
            )
            .unwrap_or(0);

        println!(
            "  Final Trust with Mara Vey:     {}",
            format_trust(mara_trust)
        );
        println!(
            "  Final Trust with Orren Vale:   {}",
            format_trust(orren_trust)
        );
        println!();

        // Check key facts
        if self
            .world
            .fact_exists(&FactScope::World, &FactId::new("PlayerToldOrrenTruth"))
        {
            println!("  ✓ You told Orren the truth");
        }
        if self
            .world
            .fact_exists(&FactScope::World, &FactId::new("PlayerLiedToOrren"))
        {
            println!("  ✗ You lied to Orren");
        }
        if self
            .world
            .fact_exists(&FactScope::World, &FactId::new("PlayerRefusedToBetrayMara"))
        {
            println!("  ✓ You kept Mara's secret");
        }

        println!();
        println!("Press Enter to exit...");
        read_line();
    }
}

fn set_hidden_fact(fact_id: &str) -> Outcome {
    Outcome::SetFact {
        scope: FactScope::Character(CharacterId::new(entities::orren())),
        fact: Fact::bool(fact_id, true),
    }
}

fn format_trust(value: i32) -> String {
    match value {
        i32::MIN..=-20 => format!("{} (Hostile)", value),
        -19..=-5 => format!("{} (Wary)", value),
        -4..=4 => format!("{} (Neutral)", value),
        5..=19 => format!("{} (Cordial)", value),
        20.. => format!("{} (Trusted)", value),
    }
}

// =============================================================================
// TUI FUNCTIONS
// =============================================================================

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

/// Renders option text with display prefixes from tags.
fn format_option_text(opt: &DialogueOption) -> String {
    // Collect display prefixes from tags that have them
    let prefixes: Vec<&str> = opt
        .tags
        .iter()
        .filter_map(|tag| tag.display_prefix())
        .collect();

    let prefix_str = prefixes.join("");

    let text = match &opt.text {
        DialogText::Inline(s) => s.as_str(),
        DialogText::LocalizedKey(key) => key.as_ref(), // Use key as fallback
    };

    if prefix_str.is_empty() {
        text.to_string()
    } else {
        format!("{} {}", prefix_str, text)
    }
}

fn render_scene(state: &GameState) {
    clear_screen();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                 THE BROKEN SEAL - Dramaturge Demo                ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");

    let node = match state.current_node() {
        Some(n) => n,
        None => {
            println!("║  Error: Node not found                                           ║");
            println!("╚══════════════════════════════════════════════════════════════════╝");
            return;
        }
    };

    let text = match &node.text {
        DialogText::Inline(s) => s.as_str(),
        DialogText::LocalizedKey(_) => "<localized>",
    };

    let speaker_name = state.get_speaker_name(node.speaker.0);
    println!("║  {:<64}║", format!("{}:", speaker_name));

    for line in word_wrap(text, 64) {
        println!("║  {:<64}║", line);
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");

    let options = state.available_options();
    if options.is_empty() {
        println!("║  No available options.                                           ║");
    } else {
        for (i, opt) in options.iter().enumerate() {
            let formatted_text = format_option_text(opt);
            let lines = word_wrap(&formatted_text, 60);
            for (j, line) in lines.iter().enumerate() {
                if j == 0 {
                    println!("║  {}. {:<60} ║", i + 1, line);
                } else {
                    println!("║     {:<60} ║", line);
                }
            }
        }
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");

    let mara_trust = state
        .world
        .get_relationship(
            entities::player(),
            entities::mara(),
            &RelationshipAxisId::new("trust"),
        )
        .unwrap_or(0);
    let orren_trust = state
        .world
        .get_relationship(
            entities::player(),
            entities::orren(),
            &RelationshipAxisId::new("trust"),
        )
        .unwrap_or(0);

    println!(
        "║  Trust: Mara={:>3}  Orren={:>3}  |  Press 'd' for debug              ║",
        mara_trust, orren_trust
    );

    if state.debug_mode {
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║  DEBUG MODE                                                      ║");
        println!(
            "║  Known facts: {:?}                                                ║",
            state.world.facts.len()
        );
        // Show available skills
        let skills: Vec<&str> = state
            .world
            .facts
            .iter()
            .filter(|sf| matches!(sf.scope, FactScope::Player))
            .filter(|sf| sf.fact.id.0.starts_with("skill:"))
            .filter(|sf| matches!(sf.fact.value, dramaturge::FactValue::Bool(true)))
            .map(|sf| sf.fact.id.0.strip_prefix("skill:").unwrap_or(&sf.fact.id.0))
            .collect();
        println!("║  Skills: {:<56} ║", skills.join(", "));
    }

    println!("╚══════════════════════════════════════════════════════════════════╝");
    print!("\nEnter choice (number): ");
    io::stdout().flush().unwrap();
}

fn word_wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            current.push_str(word);
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn handle_input(state: &mut GameState, input: &str) -> bool {
    if input.eq_ignore_ascii_case("d") {
        state.debug_mode = !state.debug_mode;
        return true;
    }
    if input.eq_ignore_ascii_case("q") {
        std::process::exit(0);
    }

    match input.parse::<usize>() {
        Ok(n) if n > 0 => {
            state.apply_option(n - 1);
            true
        }
        _ => {
            println!("Invalid input. Please enter a number.");
            false
        }
    }
}

// =============================================================================
// MAIN
// =============================================================================

fn main() {
    // Load dialogue from YAML
    let yaml_content = include_str!("dialogue.yaml");
    let yaml_graph = match YamlDialogueGraph::from_yaml(yaml_content) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to parse YAML: {}", e);
            std::process::exit(1);
        }
    };

    let graph = match yaml_graph.to_model() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to convert to model: {}", e);
            std::process::exit(1);
        }
    };

    println!("Loaded dialogue graph with {} nodes", graph.nodes.len());
    println!("Press Enter to begin...");
    read_line();

    let mut state = GameState::new(graph);

    loop {
        render_scene(&state);
        let input = read_line();
        handle_input(&mut state, &input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_loads() {
        let yaml = r#"
version: "1.0"
entry_node: 1
characters:
  player:
    id: 1
    name: "Player"
nodes:
  - id: 1
    speaker: player
    text: "Hello"
    options:
      - id: 1
        text: "End"
        target: "end"
"#;

        let yaml_graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        assert_eq!(yaml_graph.nodes.len(), 1);
    }
}
