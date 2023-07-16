use chrono::{prelude::*, Days};
use std::{
    f64::consts::PI,
    fmt::{self, Display, Formatter},
    fs,
    time::{Duration, Instant},
};

use super::schema::KmaResponseFull;

#[derive(Debug)]
pub enum KmaError {
    Http(reqwest::Error),
    Json(serde_json::Error, String),
    DateCalc(DateTime<FixedOffset>),
}

impl From<reqwest::Error> for KmaError {
    fn from(err: reqwest::Error) -> KmaError {
        KmaError::Http(err.without_url()) // Hide URL (URL has secret keys)
    }
}

impl Display for KmaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            KmaError::Http(err) => write!(f, "KmaError::Http: {}", err),
            KmaError::Json(err, _text) => write!(f, "KmaError::Json: {}", err), // Hide raw text here
            KmaError::DateCalc(dt) => write!(f, "KmaError::DateCalc: {}", dt),
        }
    }
}

pub async fn get_weather() -> Result<String, KmaError> {
    let (lat, lng) = (37.4781098, 126.9489182); // 관악구청
    let response = query_kma(lat, lng, 3).await?;

    let items = response.response.body.items.item;
    let rain_probs: Vec<String> = items
        .iter()
        .filter(|item| item.category == "POP")
        .take(12)
        .map(|item| item.fcstValue.clone() + "%")
        .collect();

    let rain_amounts: Vec<String> = items
        .iter()
        .filter(|item| item.category == "PCP")
        .take(12)
        .map(|item| {
            if item.fcstValue == "강수없음" {
                "0.0".to_string()
            } else {
                let len = item.fcstValue.len();
                item.fcstValue[..len - 2].to_string()
            }
        })
        .collect();

    let first_fcst_date = items[0].fcstDate[4..6].to_string() + "/" + &items[0].fcstDate[6..];
    let first_fcst_time = items[0].fcstTime[..2].to_string() + ":" + &items[0].fcstTime[2..];

    Ok(format!(
        "관악구청 기준, {first_fcst_date} {first_fcst_time} 이후 12시간 동안
        강수확률: [{}]
        강수량(mm): [{}]",
        rain_probs.join(", "),
        rain_amounts.join(", ")
    ))
}

async fn query_kma(lat: f64, lng: f64, num_retries: u32) -> Result<KmaResponseFull, KmaError> {
    for i in 0..num_retries {
        let result = _query_kma(lat, lng).await;
        match result {
            Ok(response) => return Ok(response),
            Err(err) => {
                if i + 1 == num_retries {
                    println!("query_kma: retry limit reached ({}/{})", i + 1, num_retries);
                    println!("{:?}", err);
                    return Err(err);
                }
                println!("query_kma: retrying ({}/{})", i + 1, num_retries);
                println!("{:?}", err);
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
    unreachable!("query_kma: unreachable!")
}

async fn _query_kma(lat: f64, lng: f64) -> Result<KmaResponseFull, KmaError> {
    let base_url = "http://apis.data.go.kr/1360000/VilageFcstInfoService_2.0/getVilageFcst";
    let service_key = fs::read_to_string(".kma_api_key")
        .expect("Should have been able to read the file")
        .trim_end()
        .to_string();
    let page_no = String::from("1");
    let num_of_rows = String::from("200");
    let data_type = String::from("JSON");
    let (base_date, base_time) = get_base_date()?;
    let (nx, ny) = dfs_xy_conv(lat, lng);

    let client = reqwest::Client::new();
    let before = Instant::now();
    let response = client
        .get(base_url)
        .query(&[
            ("serviceKey", &service_key),
            ("pageNo", &page_no),
            ("numOfRows", &num_of_rows),
            ("dataType", &data_type),
            ("base_date", &base_date),
            ("base_time", &base_time),
            ("nx", &nx.to_string()),
            ("ny", &ny.to_string()),
        ])
        .timeout(Duration::from_secs(1))
        .send()
        .await?;

    let after = Instant::now();
    println!(
        "GET {} ({}) [{} ms]",
        response.url(),
        response.status(),
        (after - before).as_millis()
    );
    let text = response.text().await?;
    serde_json::from_str::<KmaResponseFull>(&text).map_err(|err| KmaError::Json(err, text))
}

/// 위경도 -> 기상청 좌표
/// https://gist.github.com/fronteer-kr/14d7f779d52a21ac2f16 의 JS 코드를 Rust로 옮김
fn dfs_xy_conv(lat: f64, lng: f64) -> (u32, u32) {
    let sin = f64::sin;
    let cos = f64::cos;
    let tan = f64::tan;
    let ln = f64::ln;
    let powf = f64::powf;
    let floor = f64::floor;

    const RE: f64 = 6371.00877; // 지구 반경(km)
    const GRID: f64 = 5.0; // 격자 간격(km)
    const SLAT1: f64 = 30.0; // 투영 위도1(degree)
    const SLAT2: f64 = 60.0; // 투영 위도2(degree)
    const OLON: f64 = 126.0; // 기준점 경도(degree)
    const OLAT: f64 = 38.0; // 기준점 위도(degree)
    const XO: u32 = 43; // 기준점 X좌표(GRID)
    const YO: u32 = 136; // 기1준점 Y좌표(GRID)

    let deg_rad = PI / 180.0;

    let re = RE / GRID;
    let slat1 = SLAT1 * deg_rad;
    let slat2 = SLAT2 * deg_rad;
    let olon = OLON * deg_rad;
    let olat = OLAT * deg_rad;

    let mut sn = tan(PI * 0.25 + slat2 * 0.5) / tan(PI * 0.25 + slat1 * 0.5);
    sn = ln(cos(slat1) / cos(slat2)) / ln(sn);

    let mut sf = tan(PI * 0.25 + slat1 * 0.5);
    sf = powf(sf, sn) * cos(slat1) / sn;
    let mut ro = tan(PI * 0.25 + olat * 0.5);
    ro = re * sf / powf(ro, sn);

    let mut ra = tan(PI * 0.25 + lat * deg_rad * 0.5);
    ra = re * sf / powf(ra, sn);

    let mut theta = lng * deg_rad - olon;
    if theta > PI {
        theta -= 2.0 * PI;
    } else if theta < -PI {
        theta += 2.0 * PI;
    }
    theta *= sn;

    let x = floor(ra * sin(theta) + XO as f64 + 0.5) as u32;
    let y = floor(ro - ra * cos(theta) + YO as f64 + 0.5) as u32;

    (x, y)
}

fn get_base_date() -> Result<(String, String), KmaError> {
    let tz = FixedOffset::east_opt(9 * 60 * 60).unwrap();
    let current = Local::now().with_timezone(&tz); // ensure UTC+09:00
    let yesterday = current.checked_sub_days(Days::new(1)).unwrap();

    // TODO generate candidates from scratch, not modifying current time object

    // 단기예보 base_time: 0200, 0500, 0800, 1100, 1400, 1700, 2000, 2300
    // 각 base_time 기준으로 10분 이상 지난 것들 중 가장 최근인 것 선택
    let base_time_hour_candidates = vec![2, 5, 8, 11, 14, 17, 20, 23];
    let today_candidates = base_time_hour_candidates.iter().rev().map(|&hour| {
        current
            .with_hour(hour)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
    });
    let yesterday_candidates = base_time_hour_candidates.iter().rev().map(|&hour| {
        yesterday
            .with_hour(hour)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
    });
    let candidates = today_candidates.chain(yesterday_candidates);
    for cand in candidates {
        let min_diff = (current - cand).num_minutes();
        if min_diff >= 15 {
            // 15분 이상 지남
            let base_date = format!("{:04}{:02}{:02}", cand.year(), cand.month(), cand.day());
            let base_time = format!("{:02}00", cand.hour());
            return Ok((base_date, base_time));
        }
    }
    Err(KmaError::DateCalc(current))
}
