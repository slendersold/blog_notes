//! Доменный слой: структуры сущностей и перечень бизнес-ошибок.
//!
//! От остальных слоёв он не зависит: сюда не подключают HTTP или SQL.

pub mod error;
pub mod post;
pub mod user;
pub mod validation;
