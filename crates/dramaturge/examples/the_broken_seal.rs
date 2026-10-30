//! # The Broken Seal: A Dramaturge Dialogue Example (YAML-Loaded)
//!
//! This example demonstrates loading dialogue configuration from YAML files.
//! The runtime engine loads the dialogue graph at startup and executes it.
//!
//! ## Features Demonstrated
//!
//! - Loading dialogue graphs from YAML configuration
//! - Authoring-time graph validation (missing targets, unreachable nodes, dead ends)
//! - Runtime validation of conditions and outcomes
//! - Interactive TUI with debug mode
//!
//! ## Run the Example
//!
//! ```bash
//! cargo run --example the_broken_seal
//! ```

use dramaturge::*;
use dramaturge_authoring::{GraphValidator, ValidationSeverity};
use dramaturge_serde::loader::{LoadedDialogue, YamlDialogueGraph};
use std::collections::HashMap;
use std::io::{self, Write};

// =============================================================================
// CHARACTER IDS
// =============================================================================

/// Entity ids of the cast, resolved from their keys in `dialogue.yaml`.
#[derive(Clone, Copy)]
struct Entities {
    player: EntityId,
    mara: EntityId,
    orren: EntityId,
}

impl Entities {
    fn resolve(symbols: &SymbolTable) -> Self {
        let get = |key: &str| {
            symbols
                .entity(key)
                .unwrap_or_else(|| panic!("dialogue.yaml has no character '{key}'"))
        };
        Self {
            player: get("player"),
            mara: get("mara"),
            orren: get("orren"),
        }
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
    symbols: SymbolTable,
    entities: Entities,
    character_names: HashMap<EntityId, &'static str>,
    /// Which text fragments were shown; part of the save state.
    text_progress: TextProgress,
    rng: SplitMix64,
    /// The resolved text of the current node, fixed when the node is entered.
    current_text: String,
}

impl GameState {
    fn new(loaded: LoadedDialogue, has_medic: bool, has_intimidation: bool) -> Self {
        let LoadedDialogue { graph, symbols, .. } = loaded;
        let entities = Entities::resolve(&symbols);
        let mut world = DialogWorldState::new();

        // Initialize player skills based on character configuration
        // Skills are stored as facts in the Player scope with key pattern "skill:{name}"
        if has_medic {
            world.set_fact(FactScope::Player, Fact::bool("skill:medic", true));
        }
        world.set_fact(FactScope::Player, Fact::bool("skill:investigator", true));
        world.set_fact(FactScope::Player, Fact::bool("skill:streetwise", true));
        if has_intimidation {
            world.set_fact(FactScope::Player, Fact::bool("skill:intimidation", true));
        }

        // Initialize relationships
        world.add_relationship(
            entities.player,
            entities.mara,
            RelationshipAxisId::new("trust"),
            0,
        );
        world.add_relationship(
            entities.player,
            entities.orren,
            RelationshipAxisId::new("trust"),
            0,
        );
        world.add_relationship(
            entities.player,
            entities.orren,
            RelationshipAxisId::new("standing"),
            0,
        );
        world.add_relationship(
            entities.player,
            entities.mara,
            RelationshipAxisId::new("standing"),
            0,
        );

        let character_names: HashMap<EntityId, &'static str> = [
            (entities.player, "Player"),
            (entities.mara, "Mara Vey"),
            (entities.orren, "Captain Orren Vale"),
        ]
        .into_iter()
        .collect();

        let entry_node = graph
            .entry_node
            .expect("dialogue.yaml defines an entry node");

        let mut state = Self {
            world,
            current_node: entry_node,
            debug_mode: false,
            graph,
            symbols,
            entities,
            character_names,
            text_progress: TextProgress::new(),
            rng: SplitMix64::new(0x5EA1),
            current_text: String::new(),
        };
        state.resolve_current_text();
        state
    }

    /// Resolves the current node's text against the world state.
    fn resolve_current_text(&mut self) {
        let Some(node) = self.graph.nodes.iter().find(|n| n.id == self.current_node) else {
            return;
        };
        let owner = self
            .symbols
            .node_key(node.id)
            .map_or_else(|| self.symbols.display_node(node.id), str::to_string);
        let pieces = resolve_text(
            &node.text,
            &owner,
            &self.world,
            &DefaultConditionEvaluator::new(),
            &mut self.text_progress,
            &mut self.rng,
        );
        self.current_text = pieces
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
    }

    fn current_node(&self) -> Option<&DialogueNode> {
        self.graph.nodes.iter().find(|n| n.id == self.current_node)
    }

    fn available_options(&self) -> Vec<&DialogueOption> {
        let evaluator = DefaultConditionEvaluator::new();
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
                // Skills are stored as facts with key pattern "skill:{name}"
                let skill_requirements_pass = opt.tags.iter().all(|tag| {
                    if tag.requires_trait {
                        // Check if player has the required skill via fact lookup
                        let skill_fact_id = FactId::new(format!("skill:{}", tag.key));
                        matches!(
                            self.world.get_fact(&FactScope::Player, &skill_fact_id),
                            Some(FactValue::Bool(true))
                        )
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
            .get(&entity_id)
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
        let mut applier = DefaultOutcomeApplier::new();

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
                self.resolve_current_text();
            }
            DialogTarget::End => {
                self.show_ending();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    fn handle_hidden_observation(&mut self) {
        // Orren overhears Mara conversations (nodes keyed `mara.*`)
        let in_mara_scene = self
            .symbols
            .node_key(self.current_node)
            .is_some_and(|key| key.starts_with("mara."));
        if in_mara_scene {
            let mut applier = DefaultOutcomeApplier::new();
            applier.apply(
                &set_hidden_fact(self.entities.orren, "OrrenOverheardMaraConversation"),
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
                self.entities.player,
                self.entities.mara,
                &RelationshipAxisId::new("trust"),
            )
            .unwrap_or(0);
        let orren_trust = self
            .world
            .get_relationship(
                self.entities.player,
                self.entities.orren,
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
        if self.world.fact_exists(
            &FactScope::World,
            &FactId::new("PlayerGaveHalfTruthToOrren"),
        ) {
            println!("  ~ You gave Orren a half-truth");
        }
        if self
            .world
            .fact_exists(&FactScope::World, &FactId::new("PlayerBluffedOrren"))
        {
            println!("  ✗ You bluffed Orren (and failed)");
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

fn set_hidden_fact(observer: EntityId, fact_id: &str) -> Outcome {
    Outcome::SetFact {
        scope: FactScope::Character(CharacterId::new(observer)),
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

/// Capitalizes the first character of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Returns just the option text without any prefixes.
fn format_option_text_only(opt: &DialogueOption) -> String {
    match &opt.text {
        DialogText::Inline(s) => s.clone(),
        DialogText::LocalizedKey(key) => key.to_string(), // Use key as fallback
        _ => "...".to_string(),
    }
}

/// Formats the tags for display underneath an option.
/// Shows: navigation hint + explicit prefixes + inferred prefixes.
fn format_option_tags(opt: &DialogueOption) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    // 1. Navigation hint
    if let Some(hint) = &opt.navigation_hint {
        parts.push(hint.display_prefix());
    }

    // 2. All tags (explicit and inferred)
    use dramaturge::DialogTag;
    for tag in &opt.tags {
        // Explicit display prefix takes priority
        if let Some(prefix) = tag.display_prefix() {
            parts.push(prefix.to_string());
        } else {
            // Infer prefix based on axis
            let inferred = match tag.axis() {
                "skill" => Some(format!("[{}]", capitalize_first(tag.key()))),
                "moral" => Some(format!("[{}]", capitalize_first(tag.key()))),
                "speech" => Some(format!("[{}]", capitalize_first(tag.key()))),
                "social" => Some(format!("[{}]", capitalize_first(tag.key()))),
                _ => None,
            };
            if let Some(p) = inferred {
                parts.push(p);
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
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

    let text = state.current_text.as_str();

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
            // Show option text first (without prefixes)
            let text_only = format_option_text_only(opt);
            let lines = word_wrap(&text_only, 60);
            for (j, line) in lines.iter().enumerate() {
                if j == 0 {
                    println!("║  {}. {:<60} ║", i + 1, line);
                } else {
                    println!("║     {:<60} ║", line);
                }
            }

            // Show tags underneath
            if let Some(tags_str) = format_option_tags(opt) {
                let tag_lines = word_wrap(&tags_str, 58);
                for (j, line) in tag_lines.iter().enumerate() {
                    if j == 0 {
                        println!("║      → {:<58}║", line);
                    } else {
                        println!("║        {:<58}║", line);
                    }
                }
            }
        }
    }

    println!("╠══════════════════════════════════════════════════════════════════╣");

    let mara_trust = state
        .world
        .get_relationship(
            state.entities.player,
            state.entities.mara,
            &RelationshipAxisId::new("trust"),
        )
        .unwrap_or(0);
    let orren_trust = state
        .world
        .get_relationship(
            state.entities.player,
            state.entities.orren,
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
            state.world.fact_count()
        );
        // Show available skills
        let skills: Vec<&str> = state
            .world
            .facts_in(&FactScope::Player)
            .filter(|(_, value)| matches!(value, dramaturge::FactValue::Bool(true)))
            .filter_map(|(id, _)| id.0.strip_prefix("skill:"))
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
// INTRO / CHARACTER CONFIGURATION
// =============================================================================

/// Shows the intro screen and lets the player configure their character.
/// Returns the configured skills (medic, intimidation).
fn show_intro_and_configure() -> (bool, bool) {
    clear_screen();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                                                                  ║");
    println!("║                 THE BROKEN SEAL - Dramaturge Demo                ║");
    println!("║                                                                  ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                  ║");
    println!("║  Greyford. A city of stone walls, trade routes, and secrets.     ║");
    println!("║                                                                  ║");
    println!("║  You are a freelance investigator, hired to look into the        ║");
    println!("║  disappearance of medical supplies from the Free Clinic.         ║");
    println!("║  The trail has led you to Mara Vey, a clinic clerk, and          ║");
    println!("║  Captain Orren Vale, the administrator who wants answers.        ║");
    println!("║                                                                  ║");
    println!("║  But first - who are you?                                        ║");
    println!("║                                                                  ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                  ║");
    println!("║  Choose your background (determines available dialogue options): ║");
    println!("║                                                                  ║");
    println!("║    1. Street Medic - You have medical training                   ║");
    println!("║                      (Medic skill: ✓  Intimidation: ✗)           ║");
    println!("║                                                                  ║");
    println!("║    2. Enforcer     - You know how to apply pressure              ║");
    println!("║                      (Medic skill: ✗  Intimidation: ✓)           ║");
    println!("║                                                                  ║");
    println!("║    3. Negotiator   - Training in both healing and persuasion     ║");
    println!("║                      (Medic skill: ✓  Intimidation: ✓)           ║");
    println!("║                                                                  ║");
    println!("║    4. Investigator - Sharp eyes and sharper instincts            ║");
    println!("║                      (Medic skill: ✗  Intimidation: ✗)           ║");
    println!("║                                                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    print!("\nSelect background (1-4): ");
    io::stdout().flush().unwrap();

    loop {
        let input = read_line();
        match input.as_str() {
            "1" => {
                println!("\nYou chose: Street Medic");
                println!("Press Enter to begin...");
                read_line();
                return (true, false);
            }
            "2" => {
                println!("\nYou chose: Enforcer");
                println!("Press Enter to begin...");
                read_line();
                return (false, true);
            }
            "3" => {
                println!("\nYou chose: Negotiator");
                println!("Press Enter to begin...");
                read_line();
                return (true, true);
            }
            "4" => {
                println!("\nYou chose: Investigator");
                println!("Press Enter to begin...");
                read_line();
                return (false, false);
            }
            _ => {
                print!("Invalid choice. Please enter 1, 2, 3, or 4: ");
                io::stdout().flush().unwrap();
            }
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

    let loaded = match yaml_graph.to_model() {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("Failed to convert to model: {}", e);
            std::process::exit(1);
        }
    };
    for warning in &loaded.warnings {
        eprintln!("  [WARN]  {warning}");
    }

    // Validate graph structure before running; messages name nodes by key
    let issues = GraphValidator::new().validate_with_symbols(&loaded.graph, &loaded.symbols);
    if !issues.is_empty() {
        eprintln!("Graph validation:");
        for issue in &issues {
            let prefix = match issue.severity {
                ValidationSeverity::Error => "  [ERROR]",
                ValidationSeverity::Warning => "  [WARN] ",
                ValidationSeverity::Info => "  [INFO] ",
            };
            eprintln!("{} {}", prefix, issue.message);
        }
        if issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Error)
        {
            eprintln!("Aborting: graph has validation errors.");
            std::process::exit(1);
        }
        eprintln!();
    }

    // Show intro and configure character
    let (has_medic, has_intimidation) = show_intro_and_configure();

    // Create game state with configured skills
    let mut state = GameState::new(loaded, has_medic, has_intimidation);

    loop {
        render_scene(&state);
        let input = read_line();
        handle_input(&mut state, &input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn test_yaml_loads() {
        let yaml = r#"
version: "2.0"
entry_node: hello
characters:
  player:
    name: "Player"
nodes:
  - id: hello
    speaker: player
    text: "Hello"
    options:
      - id: hello.end
        text: "End"
        target: end
"#;

        let yaml_graph = YamlDialogueGraph::from_yaml(yaml).expect("Should parse");
        check!(yaml_graph.nodes.len() == 1);
    }

    #[test]
    fn bundled_dialogue_loads_with_keys_and_validates() {
        let loaded = YamlDialogueGraph::from_yaml(include_str!("dialogue.yaml"))
            .expect("Should parse")
            .to_model()
            .expect("Should convert");
        check!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        let entry = loaded.graph.entry_node.unwrap();
        check!(loaded.symbols.node_key(entry) == Some("mara.intro"));
        Entities::resolve(&loaded.symbols);

        let issues = GraphValidator::new().validate_with_symbols(&loaded.graph, &loaded.symbols);
        check!(
            !issues
                .iter()
                .any(|i| i.severity == ValidationSeverity::Error),
            "{issues:?}"
        );
    }
}
