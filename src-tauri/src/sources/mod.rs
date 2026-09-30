//! sources — les sources de fiches (specs/instructions/agnostique-mqtt.md) : le bus MQTT local
//! (`bus`, agnostique de tout runner), la lecture des fichiers de sessions et de fiches du disque
//! (`local_sessions`, « Ce poste » — ce-poste-vue-locale-lisibilite.md D1/D2), l'adaptateur Claude
//! Code par transcripts (`claude_code_files`, réduit au repli par session, D3) et la source
//! distante (`remote`). Le cœur (`crate::tree`) ne connaît que le type `crate::card::AgentCard`.

pub mod bus;
pub mod claude_code_files;
pub mod local_sessions;
pub mod remote;
