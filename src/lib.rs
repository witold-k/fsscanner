// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

pub mod fileentry;
pub mod fsscanner_base;
pub mod fsscanner_mt;
pub mod fsscanner_st;
pub mod pathfilter;
pub mod pathutils;
pub mod threadpool;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

