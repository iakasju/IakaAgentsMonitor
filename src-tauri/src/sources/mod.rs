//! sources — les DEUX sources de fiches (specs/instructions/agnostique-mqtt.md) : le bus MQTT
//! (`bus`, agnostique de tout runner) et l'adaptateur Claude Code par lecture de fichiers
//! (`claude_code_files`, repli strict quand le broker est absent, Q3-A). Le cœur (`crate::tree`)
//! ne connaît que le type `crate::card::AgentCard` que les deux rendent.

pub mod bus;
pub mod claude_code_files;
