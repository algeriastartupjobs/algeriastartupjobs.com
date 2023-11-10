use bk_tree::{metrics, BKTree};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, QueryBuilder, Row, Sqlite};
use std::sync::{Arc, Mutex};

use crate::{
  _utils::{
    error::SearchError,
    string::{escape_double_quote, get_searchable_words, get_words},
  },
  account::model::{AccountNameTrait, CompactAccount},
  post::model::Post,
  tag::model::CompactTag,
};

#[derive(Debug, Serialize, Deserialize)]
struct WordIndex {
  word: String,
  model_id: u32,
  appear_in: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WordRecord {
  word: String,
}

pub struct SearchService {
  search_sql_db: Arc<Pool<Sqlite>>,
  bk_tree: Arc<Mutex<BKTree<String>>>,
}

impl SearchService {
  pub fn new(search_sql_db: Arc<Pool<Sqlite>>) -> Self {
    Self {
      search_sql_db,
      bk_tree: Arc::new(Mutex::new(BKTree::new(metrics::Levenshtein))),
    }
  }

  pub async fn refresh_bk_tree(&self) -> Result<(), SearchError> {
    let conn = self.search_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!(
        "Error while getting sql connection to refresh bk tree: {:?}",
        conn
      );
      return Err(SearchError::InternalError);
    }
    let mut conn = conn.unwrap();

    // @TODO-ZM: figure out how query $ replacement work, there is some unneeded "magic" here
    let db_result = sqlx::query(
      r#"
        SELECT DISTINCT word
        FROM word
        WHERE appear_in IN ('post_title', 'post_short_description', 'post_tag_name', 'post_poster_display_name');
      "#,
    )
    .fetch_all(&mut *conn)
    .await;

    if db_result.is_err() {
      tracing::error!("Error while getting all words: {:?}", db_result.err());
      return Err(SearchError::InternalError);
    }
    let db_result = db_result.unwrap();

    let mut words = vec![];

    for row in db_result {
      words.push(row.get::<String, _>("word"));
    }

    let bk_tree = self.bk_tree.lock();
    if bk_tree.is_err() {
      tracing::error!("Error while getting bk tree lock: {:?}", bk_tree.err());
      return Err(SearchError::InternalError);
    }
    let mut bk_tree = bk_tree.unwrap();
    // @TODO-ZM: clean the tree before adding new words

    for word in words {
      bk_tree.add(word);
    }

    Ok(())
  }

  pub async fn index_posts(
    &self,
    posts: Vec<Post>,
    tags: Vec<CompactTag>,
    posters: Vec<CompactAccount>,
  ) -> Result<(), SearchError> {
    let mut word_indexes: Vec<WordIndex> = vec![];
    for post in posts {
      // @TODO-ZM: use regex \d to split the string
      get_words(&post.title).for_each(|word| {
        let word = word.to_lowercase();

        word_indexes.push(WordIndex {
          word,
          model_id: post.id,
          appear_in: "post_title".to_string(),
        });
      });

      get_words(&post.description).for_each(|word| {
        let word = word.to_lowercase();

        word_indexes.push(WordIndex {
          word,
          model_id: post.id,
          appear_in: "post_description".to_string(),
        });
      });

      for tag_id in post.tag_ids {
        let tag = tags
          .iter()
          .find(|tag| tag.id == tag_id).cloned();

        if tag.is_none() {
          tracing::error!("Failed to find the tag by id {}", tag_id,);
          return Err(SearchError::InternalError);
        };
        let tag = tag.unwrap();

        get_words(&tag.name).for_each(|word| {
          let word = word.to_lowercase();

          word_indexes.push(WordIndex {
            word,
            model_id: post.id,
            appear_in: "post_tag_name".to_string(),
          });
        });
      }

      let poster = posters.iter().find(|poster| poster.id == post.poster_id);
      if poster.is_none() {
        tracing::error!("Failed to find the poster");
        return Err(SearchError::InternalError);
      }
      let poster = poster.unwrap();

      get_words(&poster.get_display_name()).for_each(|word| {
        let word = word.to_lowercase();

        word_indexes.push(WordIndex {
          word,
          model_id: post.id,
          appear_in: "post_poster_display_name".to_string(),
        });
      });
    }

    let conn = self.search_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection to index: {:?}", conn);
      return Err(SearchError::InternalError);
    }
    let mut conn = conn.unwrap();

    let mut query_builder =
      QueryBuilder::new("INSERT INTO word (word, model_type, model_id, appear_in) ");

    query_builder.push_values(word_indexes, |mut b, new_word_index| {
      b.push_bind(new_word_index.word)
        .push_bind("post")
        .push_bind(new_word_index.model_id)
        .push_bind(new_word_index.appear_in);
    });

    let db_result = query_builder.build().execute(&mut *conn).await;

    if db_result.is_err() {
      tracing::error!("Error while indexing posts: {:?}", db_result);
      return Err(SearchError::InternalError);
    }

    Ok(())
  }

  fn get_corrected_queries(&self, query: &String, max_suggestions: u8) -> Vec<String> {
    let query_words = get_searchable_words(query);

    let bk_tree = self.bk_tree.lock().unwrap();
    let mut corrected_words_in_queries = vec![];
    for query_word in &query_words {
      let mut corrected_words_with_distance = vec![];

      let tolerance = query_word.len() / 2;
      bk_tree
        .find(query_word, tolerance as u32)
        .for_each(|(distance, corrected_word)| {
          corrected_words_with_distance.push((distance, corrected_word));
        });

      corrected_words_with_distance.sort_by_key(|k| k.0);

      corrected_words_in_queries.push(
        corrected_words_with_distance
          .iter()
          .map(|(_, corrected_word)| corrected_word)
          .map(|corrected_word| escape_double_quote(corrected_word))
          .collect::<Vec<String>>(),
      );
    }

    let mut corrected_queries = vec![];

    for index in 0..max_suggestions {
      let mut corrected_query_words = vec![];
      for query_word_index in 0..query_words.len() {
        let corrected_word = corrected_words_in_queries
          .get(query_word_index)
          .unwrap()
          .get(index as usize);

        match corrected_word {
          Some(corrected_word) => corrected_query_words.push(corrected_word.as_str()),
          None => corrected_query_words.push({
            match query_words.get(query_word_index) {
              Some(query_word) => query_word,
              None => "",
            }
          }),
        }
      }
      corrected_queries.push(corrected_query_words.join(" "));
    }

    // filter out the original query
    let corrected_queries = corrected_queries
      .iter()
      .filter(|corrected_query| *corrected_query != query)
      .map(|corrected_query| corrected_query.to_string())
      .collect::<Vec<String>>();

    corrected_queries
  }

  // @TODO-ZM: add pagination
  pub async fn search_posts(&self, query: &String) -> Result<Vec<u32>, SearchError> {
    let conn = self.search_sql_db.acquire().await;
    if conn.is_err() {
      tracing::error!("Error while getting sql connection to search: {:?}", conn);
      return Err(SearchError::InternalError);
    }
    let mut conn = conn.unwrap();

    let mut search_queries = self.get_corrected_queries(query, 3);
    search_queries.insert(0, query.clone());
    let search_queries_count = search_queries.len();

    let query = search_queries
      .iter()
      .enumerate()
      .map(|(index, search_query)| {
        format!(
          r#"
          SELECT model_id, SUM(count * weight * {}) AS score
          FROM (
            SELECT id, word, model_type, model_id, appear_in, count(*) AS count,
            CASE
              WHEN appear_in = 'post_title' THEN 100
              WHEN appear_in = 'post_poster_display_name' THEN 50
              WHEN appear_in = 'post_short_description' THEN 25
              WHEN appear_in = 'post_tag_name' THEN 5
              ELSE 1
            END AS weight
            FROM word
            WHERE word In ({})
            GROUP BY word, model_type, model_id, appear_in
          ) AS sub
          GROUP BY model_id
          ORDER BY score DESC;
          "#,
          search_queries_count - index,
          // @TODO-ZM: Potential SQL injection vulnerability!
          search_query
            .split(' ')
            .map(|word| format!("'{}'", word))
            .join(", "),
        )
      })
      .collect::<Vec<String>>()
      .join("\n");

    let db_results = sqlx::query(&query).fetch_all(&mut *conn).await;

    if db_results.is_err() {
      tracing::error!("Error while searching posts: {:?}", db_results.err());
      return Err(SearchError::InternalError);
    }
    let db_results = db_results.unwrap();

    let mut model_ids_with_scores = vec![];

    for row in db_results {
      model_ids_with_scores.push((row.get::<u32, _>("model_id"), row.get::<i64, _>("score")));
    }

    // aggregate the scores for the same model_id
    let mut model_ids_with_scores = model_ids_with_scores
      .into_iter()
      .sorted_by_key(|(model_id, _)| *model_id)
      .group_by(|(model_id, _)| *model_id)
      .into_iter()
      .map(|(model_id, group)| {
        let mut score = 0;
        for (_, group_score) in group {
          score += group_score;
        }
        (model_id, score)
      })
      .collect::<Vec<(u32, i64)>>();

    model_ids_with_scores.sort_by_key(|(_, score)| -score);

    let model_ids_sorted = model_ids_with_scores
      .iter()
      .map(|(model_id, _)| *model_id)
      .collect::<Vec<u32>>();

    Ok(model_ids_sorted)
  }
}
