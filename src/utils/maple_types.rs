use core::fmt;
use std::fmt::{Display, Formatter};

use scraper::{Html, Selector};

fn from_selector(document: &Html, selector_string: &str) -> String {
    fn query(document: &Html, selector_string: &str) -> eyre::Result<String> {
        let selector =
            Selector::parse(&selector_string).map_err(|_| eyre::eyre!("Selector parsing error"))?;

        let result = document
            .select(&selector)
            .flat_map(|element| element.text().collect::<Vec<_>>())
            .collect::<Vec<_>>()
            .first()
            .ok_or_else(|| eyre::eyre!("Nothing matches with given selector"))?
            .replace("\n", "")
            .replace("\t", "")
            .trim()
            .to_string();

        Ok(result)
    }

    let result = query(document, selector_string);
    match result {
        Ok(result) => result,
        Err(_) => String::from("N/A"),
    }
}

#[derive(Debug)]
pub struct MapleUser {
    name: String,
    job: String,
    character_level: String,
    union_level: String,
    mureung_score: String,
}

impl MapleUser {
    pub fn from(document: Html) -> Self {
        let name = from_selector(
            &document,
            "#user-profile > section > div.row.row-normal > div.col-lg-8 > div > h3 > b",
        );
        let job = from_selector(&document, "#user-profile > section > div.row.row-normal > div.col-lg-8 > div > div.user-summary > ul > li:nth-child(2)");
        let character_level = from_selector(&document, "#user-profile > section > div.row.row-normal > div.col-lg-8 > div > div.user-summary > ul > li:nth-child(1)");
        let union_level = from_selector(&document, "#app > div.card.border-bottom-0 > div > section > div.row.text-center > div:nth-child(3) > section > div > div > span");
        let mureung_score = from_selector(&document, "#app > div.card.border-bottom-0 > div > section > div.row.text-center > div:nth-child(1) > section > div > div.pt-4.pt-sm-3.pb-4 > div > h1").replace(" ", "");

        Self {
            name,
            job,
            character_level,
            union_level,
            mureung_score,
        }
    }
}

impl Display for MapleUser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "캐릭터명: {}\n직업: {}\n캐릭터 레벨: {}\n유니온 레벨: {}\n무릉도장: {}",
            self.name, self.job, self.character_level, self.union_level, self.mureung_score
        )
    }
}
