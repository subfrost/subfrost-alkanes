pub mod precompiled {
    pub mod fr_btc_build;
}

#[cfg(test)]
pub mod tests {
    pub mod fr;
    pub mod helpers;
    pub mod payment_tests;
    pub mod core_tests;
}
