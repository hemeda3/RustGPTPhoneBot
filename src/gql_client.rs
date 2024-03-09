use std::collections::HashMap;
use csv::Writer;
use log::error;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::rs_generic_types::{GraphQLQuery, GraphQLResponse};


pub async fn search_products_function(array_data: Vec<String>) -> Option<String> {


    let query = String::from(r#"
    query GetSearchData(
    $queryParam: String
    $page: Int
    $pageSize: Int
    $sort: String
  ) {
    searchData(
      queryParam: $queryParam
      page: $page
      pageSize: $pageSize
      sort: $sort
    ) {
      ...ProductList
    }
  }

  fragment ProductList on ProductList {
    categoryType
    totalFiltersCount
    products {
      brandName
      name
      isOnSale
      badges {
        uid
        name
        type
      }
      isOutOfStock
      id
      optimumPoints {
        base
        bonus
        optimumPromoText
      }

      id
      price {
        formattedPrice
      }
      salePrice {
        formattedPrice
      }
    }
    pagination {
      currentPage
      pageSize
      totalPages
      totalResults
    }
  }
"#);
    let client = Client::new();

    let query = String::from(query);
    let mut variables = HashMap::new();
    variables.insert("queryParam".to_string(), Value::String(array_data.join(":")));
    variables.insert("page".to_string(), Value::from(0));
    variables.insert("pageSize".to_string(), Value::from(10));
    variables.insert("sort".to_string(), Value::from("asc"));

    let graphql_query = GraphQLQuery { query, variables };

    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers.insert("x-lcl-apikey", HeaderValue::from_static(""));
    headers.insert("content-type", HeaderValue::from_static("application/json"));
    headers.insert("origin_session_header", HeaderValue::from_static(""));
    let client_response = client
        .post("https://api.shoppersdrugmart.ca/ecommerce")
        .headers(headers)
        .json(&graphql_query)
        .send()
        .await;
    match client_response {
        Ok(res) => {
            let res_value: Result<Value, reqwest::Error> = res.json().await;
            match &res_value {
                Ok(value) => println!("Raw JSON: {}", value),
                Err(e) => println!("Failed to get raw JSON: {:?}", e),
            }

            match res_value {
                Ok(val) => match serde_json::from_value::<GraphQLResponse>(val) {
                    Ok(json_res) => {
                        let mut wtr = Writer::from_writer(vec![]);
                        wtr.write_record(&["Brand", "ID", "IsOnSale", "IsOutOfStock", "Name", "BasePoints", "BonusPoints", "FormattedPrice"]).unwrap();

                        if let Some(data) = json_res.data {
                            if let Some(search_data) = data.searchData {
                                if let Some(products) = search_data.products {
                                    for product in products {
                                        let record = vec![
                                            product.brandName.unwrap_or_default(),
                                            product.id.unwrap_or_default(),
                                            product.isOnSale.map_or(String::from("N/A"), |b| b.to_string()),
                                            product.isOutOfStock.map_or(String::from("N/A"), |b| b.to_string()),
                                            product.name.unwrap_or_default(),
                                            product.optimumPoints.as_ref().map_or(String::from("N/A"), |p| p.base.map_or(String::from("N/A"), |b| b.to_string())),
                                            product.optimumPoints.as_ref().map_or(String::from("N/A"), |p| p.bonus.map_or(String::from("N/A"), |b| b.to_string())),
                                            product.price.as_ref().map_or(String::from("N/A"), |p| p.formattedPrice.clone().unwrap_or(String::from("N/A"))),
                                        ];
                                        wtr.write_record(&record).unwrap();
                                    }
                                }
                            }
                        }
                        return Some(String::from_utf8(wtr.into_inner().unwrap()).unwrap());
                    },
                    Err(e) => println!("Serde error: {:?}", e),
                },
                Err(e) => println!("Reqwest error: {:?}", e),
            }
        },
        Err(e) => {
            println!("Failed to get CSV data s2 {:?}", e);
        },
    }
    None
}