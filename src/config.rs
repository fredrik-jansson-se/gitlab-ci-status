use anyhow::Context;

fn default_num_pipelines() -> usize {
    5
}

fn default_match_branch() -> regex::Regex {
    regex::Regex::new(".*").unwrap()
}

fn de_match_branch<'de, D>(de: D) -> Result<regex::Regex, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = regex::Regex;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer between -2^31 and 2^31")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            regex::Regex::new(v).map_err(|e| E::custom(e))
        }
    }
    de.deserialize_str(Visitor)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Config {
    pub gitlab_access_token: String,

    pub projects: Vec<Project>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Project {
    pub name: String,
    #[serde(deserialize_with = "de_match_branch")]
    #[serde(default = "default_match_branch")]
    pub match_branch_re: regex::Regex,
    #[serde(default = "default_num_pipelines")]
    pub num_pipelines: usize,
}

pub(crate) fn load_config(cfg_file: &str) -> anyhow::Result<Config> {
    let cfg = std::fs::read(cfg_file).context(cfg_file.to_string())?;

    let config: Config = serde_yaml::from_slice(&cfg)?;

    Ok(config)
}
