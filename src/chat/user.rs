#[derive(Debug)]
pub struct User {
    pub name: String,
}

impl User {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        User::validate(name)?;
        Ok(Self { name: name.into() })
    }

    fn validate(name: &str) -> anyhow::Result<()> {
        if name == "" {
            return Err(anyhow::anyhow!("Will not accept blank name"));
        }
        if !name.chars().all(|ch| ch.is_alphanumeric()) {
            return Err(anyhow::anyhow!("Not Alphanumeric"));
        }
        if name.len() > 16 {
            return Err(anyhow::anyhow!("Name is too long"));
        }
        Ok(())
    }
}
