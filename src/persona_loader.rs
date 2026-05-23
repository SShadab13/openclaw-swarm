use std::fs;
use anyhow::{Result, Context};
use crate::models::{Persona, Personality};

pub struct Loader;

impl Loader {
    pub fn load_personas(dir: &str) -> Result<Vec<Persona>> {
        let mut personas = Vec::new();
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                
                let persona: Persona = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML in {}", path.display()))?;
                
                personas.push(persona);
            }
        }
        
        Ok(personas)
    }
    
    pub fn load_personalities(dir: &str) -> Result<Vec<Personality>> {
        let mut personalities = Vec::new();
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                
                let personality: Personality = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML in {}", path.display()))?;
                
                personalities.push(personality);
            }
        }
        
        Ok(personalities)
    }
    
    pub fn get_persona_by_id(personas: &[Persona], id: &str) -> Option<Persona> {
        personas.iter().find(|p| p.id == id).cloned()
    }
    
    pub fn get_personality_by_id(personalities: &[Personality], id: &str) -> Option<Personality> {
        personalities.iter().find(|p| p.id == id).cloned()
    }
}
