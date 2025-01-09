//! - [A Pattern-Based Approach to Hyponymy Relation Acquisition for the Agricultural Thesaurus.](https://www.kl.itc.nagoya-u.ac.jp/person/mnakamur/Research/aos2012sep/AIW2012_AOS2012_Proceedings_pp2-9.pdf)
//! - [法令文中において括弧書きで定義されている法令用語とその語釈文の抽出](https://www.anlp.jp/proceedings/annual_meeting/2013/pdf_dir/P4-6.pdf)
//!
//! を元に拡張した提案手法
//!
//! -「この法律において「〜」とは、〜をいう。」というパターン
//! - 「～に規定する～をいう」で取得するパターン
//! - 読点・「又は」・「若しくは」・「及び」・「並びに」などの並列表現が出現するまで取得するパターン（ただし、読点の直前が漢字の場合は並列表現なのでそのあとも取得する）
//!

use regex::Regex;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Abb {
  pub formal: String,
  pub abbr: String,
  pub in_paren: bool,
}

/// 括弧を除去した際の情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveParenInfo {
  /// 括弧の出現位置
  pub index: usize,
  /// 括弧内のテキストのうち
  pub sub_text: String,
}

pub fn remove_paren(text: &str) -> (String, Vec<RemoveParenInfo>) {
  let mut count = 0_usize;
  let mut depth = 0_usize;
  let mut s = String::new();
  let mut s_in_paren = String::new();
  let mut v = Vec::new();
  //let re1 = regex::Regex::new(
  //  "^.*(以下|において)([^「」]+)?「(?<s>[^「」]+)」(と総称する。|という。|といい、|とする。)$",
  //)
  //.unwrap();
  //let re2 = regex::Regex::new("(.+。)?(?<s>[^。]+)(をいう。|をいい、).*").unwrap();
  for c in text.chars() {
    if c == '（' {
      if depth != 0 {
        s_in_paren.push(c);
      }
      depth += 1;
    } else if c == '）' {
      if depth != 1 {
        s_in_paren.push(c);
      }
      if depth == 1 {
        v.push(RemoveParenInfo {
          index: count,
          sub_text: s_in_paren.clone(),
        });
        s_in_paren = String::new();
      }
      depth -= 1;
    } else if depth == 0 {
      s.push(c);
      count += 1;
    } else {
      s_in_paren.push(c);
    }
  }
  (s, v)
}

pub fn analysis_abbr(
  removed_paren_text: &str,
  remove_paren_info_list: &[RemoveParenInfo],
  is_in_paren: bool,
) -> Vec<Abb> {
  let mut v = Vec::new();
  let re1 = Regex::new("「(?<abbr>[^「」]+)」とは、(?<formal>[^。]+)をいう。").unwrap();
  if let Some(caps) = re1.captures(removed_paren_text) {
    let abbr = &caps["abbr"];
    let formal = &caps["formal"];
    v.push(Abb {
      formal: formal.to_string(),
      abbr: abbr.to_string(),
      in_paren: is_in_paren,
    })
  }
  let re2 = Regex::new("(?<formal>[^。]+に規定する(?<abbr>[^。、]+))をいう。").unwrap();
  let re3 = regex::Regex::new(
    "^.*(以下|において)([^「」]+)?「(?<s>[^「」]+)」(と総称する。|という。|といい、|とする。)$",
  )
  .unwrap();
  let re4 = regex::Regex::new("(.+。)?(?<s>[^。]+)(をいう。|をいい、).*").unwrap();
  for info in remove_paren_info_list.iter() {
    let subtext = &info.sub_text;
    // 括弧の中の文字列にも再帰的に適用
    let (sub_removed_text, sub_remove_info_list) = crate::remove_paren(subtext);
    let sublist = analysis_abbr(&sub_removed_text, &sub_remove_info_list, true);
    v.extend(sublist);
    if let Some(caps) = re2.captures(&sub_removed_text) {
      let formal = &caps["formal"];
      let abbr = &caps["abbr"];
      v.push(Abb {
        formal: formal.to_string(),
        abbr: abbr.to_string(),
        in_paren: is_in_paren,
      })
    } else {
      let re3_s_opt = re3.captures(&sub_removed_text);
      let re4_s_opt = re4.captures(&sub_removed_text);
      if re3_s_opt.is_some() || re4_s_opt.is_some() {
        let mut text = removed_paren_text
          .chars()
          .take(info.index)
          .collect::<Vec<char>>();
        text.reverse();
        let size = text.len();
        let mut s = Vec::new();
        // 「○○等」を検出する
        let mut is_tou = false;
        if text.first() == Some(&'等') && re4_s_opt.is_some() {
          let caps = re4.captures(&sub_removed_text).unwrap();
          let formal = &caps["s"];
          let mut chars = text.iter();
          chars.next();
          for c in chars {
            let mut new_s = s.clone();
            new_s.push(*c);
            new_s.reverse();
            let abbr = new_s.iter().collect::<String>();
            if !formal.contains(&abbr) {
              break;
            } else {
              s.push(*c);
            }
          }
          if !s.is_empty() {
            s.insert(0, '等');
            is_tou = true;
          }
        }
        if !is_tou {
          // 「○○等」ではなかった場合
          for (i, c) in text.iter().enumerate() {
            match *c {
              '、' => {
                if i + 1 != size
                  && !((text[i + 1] >= '\u{3041}' && text[i + 1] <= '\u{3094}')
                    || (text[i + 1] >= '\u{30A1}' && text[i + 1] <= '\u{30FA}'))
                {
                  // 「漢字、」なのでそのまま続ける
                  s.push(*c);
                } else {
                  break;
                }
              }
              _ => {
                s.push(*c);
              }
            }
          }
        }
        s.reverse();
        let s = s.iter().collect::<String>();
        if let Some(caps) = re3_s_opt {
          let abbr = caps["s"].to_string();
          v.push(Abb {
            formal: s.clone(),
            abbr,
            in_paren: is_in_paren,
          })
        }
        if let Some(caps) = re4_s_opt {
          let formal = caps["s"].to_string();
          v.push(Abb {
            formal,
            abbr: s,
            in_paren: is_in_paren,
          })
        }
      }
    }
  }
  v
}

fn main() {}
