#![allow(non_snake_case)]
#![allow(dead_code)]

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct KmaResponseFull {
    pub response: KmaResponse,
}

#[derive(Deserialize, Debug)]
pub struct KmaResponse {
    header: KmaHeader,
    pub body: KmaBody,
}

#[derive(Deserialize, Debug)]
pub struct KmaHeader {
    resultCode: String,
    resultMsg: String,
}

#[derive(Deserialize, Debug)]
pub struct KmaBody {
    dataType: String,
    pub items: KmaItems,
    pageNo: u32,
    numOfRows: u32,
    totalCount: u32,
}

#[derive(Deserialize, Debug)]
pub struct KmaItems {
    pub item: Vec<KmaItem>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct KmaItem {
    pub baseDate: String,
    pub baseTime: String,
    pub category: String,
    pub fcstDate: String,
    pub fcstTime: String,
    pub fcstValue: String,
    nx: u32,
    ny: u32,
}
