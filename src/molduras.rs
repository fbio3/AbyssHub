//! Molduras da foto de perfil. Quem ganha um evento recebe a moldura que o criador escolheu
//! para aquela colocação (o servidor entrega quando o evento acaba).

/// (id, nome, como obter), da mais rara para a mais comum.
pub const TODAS: [(&str, &str, &str); 3] = [
    ("ruby", "Ruby", "Primeiro lugar em eventos"),
    ("diamante", "Diamante", "Segundo lugar em eventos"),
    ("bronze", "Bronze", "Terceiro lugar em eventos"),
];

pub fn existe(id: &str) -> bool {
    TODAS.iter().any(|m| m.0 == id)
}

/// "Ruby", "Diamante"... (o próprio id se não conhecer).
pub fn nome(id: &str) -> &str {
    TODAS.iter().find(|m| m.0 == id).map(|m| m.1).unwrap_or(id)
}

pub fn descricao(id: &str) -> &'static str {
    TODAS.iter().find(|m| m.0 == id).map(|m| m.2).unwrap_or("")
}
