//! Demo lecture for the UI lab. Every cue, error and gloss here is invented.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    /// A spelling the user already confirmed earlier (their own memory).
    Memory,
    /// Sounds almost the same as a known term.
    Sound,
    /// Inferred from context; the least certain.
    Possible,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Tier::Memory => "你之前確認過的寫法",
            Tier::Sound => "聽起來很像",
            Tier::Possible => "可能有誤（AI 推測）",
        }
    }
}

pub struct Cue {
    pub start: f32,
    pub end: f32,
    pub text: &'static str,
}

pub struct Issue {
    pub cue: usize,
    pub range: (usize, usize),
    pub heard: &'static str,
    pub suggestion: &'static str,
    pub tier: Tier,
    pub reason: &'static str,
}

pub struct Gloss {
    pub cue: usize,
    pub range: (usize, usize),
    pub term: &'static str,
    pub zh: &'static str,
    pub en: &'static str,
    pub confirmed: bool,
}

fn span(text: &str, needle: &str) -> (usize, usize) {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} not in {text}"));
    (start, start + needle.len())
}

pub struct Lecture {
    pub title: &'static str,
    pub teacher: &'static str,
    pub cues: Vec<Cue>,
    pub issues: Vec<Issue>,
    pub glosses: Vec<Gloss>,
}

impl Lecture {
    pub fn duration(&self) -> f32 {
        self.cues.last().map_or(0.0, |c| c.end)
    }
    pub fn cue_at(&self, t: f32) -> usize {
        self.cues.iter().rposition(|c| c.start <= t).unwrap_or(0)
    }
}

pub fn demo() -> Lecture {
    let t: [(f32, f32, &'static str); 14] = [
        (0.0, 5.0, "好，各位同學我們開始，上週我們講了踢度下架。"),
        (
            5.0,
            11.0,
            "今天要把它講完，先簡單暖個身，上週有沒有人回去自己試過？",
        ),
        (
            11.0,
            19.0,
            "梯度下降其實很簡單，你就把它想成下山，每一步都往最陡的方向走。",
        ),
        (
            19.0,
            26.0,
            "學習率如果設太大，你就會一直跳過頭，模型就沒辦法收獵。",
        ),
        (
            26.0,
            33.0,
            "那我們用白頭之來實作，裡面的 TMIcer 會幫你更新參數。",
        ),
        (
            33.0,
            40.0,
            "它背後做的事情就是 BackPrepitation，也就是反向傳播。",
        ),
        (
            40.0,
            47.0,
            "學習率你要是不確定，先設小一點，你就把它ㄍㄧㄥ住不要亂動。",
        ),
        (47.0, 54.0, "跑完以後把 loss curve 畫出來，看看有沒有收獵。"),
        (
            54.0,
            60.0,
            "如果曲線一直抖，通常就是學習率太大，不是模型的問題。",
        ),
        (
            60.0,
            67.0,
            "下中要交作業，截止時間是星期五晚上十一點五十九分。",
        ),
        (67.0, 73.0, "作業會檢查你有沒有真的收練，而不是只看分數。"),
        (73.0, 80.0, "有問題的話可以寄信給助教，也可以直接來問我。"),
        (
            80.0,
            86.0,
            "好，那今天的重點就是這三件事：學習率、收斂、優化器。",
        ),
        (86.0, 92.0, "下課前我們來看一個例子，大家把筆電打開。"),
    ];
    let cues: Vec<Cue> = t
        .iter()
        .map(|&(start, end, text)| Cue { start, end, text })
        .collect();
    let mk = |cue: usize,
              heard: &'static str,
              suggestion: &'static str,
              tier: Tier,
              reason: &'static str| Issue {
        cue,
        range: span(cues[cue].text, heard),
        heard,
        suggestion,
        tier,
        reason,
    };
    let issues = vec![
        mk(
            0,
            "踢度下架",
            "梯度下降",
            Tier::Sound,
            "唸起來幾乎和這堂課的詞「梯度下降」一樣，而且後面接著「今天繼續把它講完」。",
        ),
        mk(
            3,
            "收獵",
            "收斂",
            Tier::Sound,
            "「模型沒辦法收…」後面接收斂最通順，音也很接近。",
        ),
        mk(
            4,
            "白頭之",
            "PyTorch",
            Tier::Possible,
            "前後文在講用工具「實作」，而「白頭之」唸起來像 PyTorch。這是我根據語意猜的，請你聽一下。",
        ),
        mk(
            4,
            "TMIcer",
            "optimizer",
            Tier::Sound,
            "拼音很像 optimizer，而且後面說「更新參數」。",
        ),
        mk(
            5,
            "BackPrepitation",
            "backpropagation",
            Tier::Sound,
            "拼音很像 backpropagation，而且前後文在講反向傳播。",
        ),
        mk(
            7,
            "收獵",
            "收斂",
            Tier::Sound,
            "和第 1 個「收獵」一樣的寫法。",
        ),
        mk(
            9,
            "下中",
            "下週",
            Tier::Possible,
            "後面在講作業截止時間，「下週」比「下中」通順。這是 AI 推測。",
        ),
        mk(
            10,
            "收練",
            "收斂",
            Tier::Sound,
            "「收練」唸起來和「收斂」一樣，而且前面在講模型有沒有真的學好。",
        ),
    ];
    let gl =
        |cue: usize, term: &'static str, zh: &'static str, en: &'static str, confirmed: bool| {
            Gloss {
                cue,
                range: span(cues[cue].text, term),
                term,
                zh,
                en,
                confirmed,
            }
        };
    let glosses = vec![
        gl(
            1,
            "暖個身",
            "先簡單複習或熱身一下",
            "do a quick warm-up first",
            false,
        ),
        gl(
            6,
            "ㄍㄧㄥ住",
            "固定住、維持不變",
            "hold it fixed; do not change it",
            true,
        ),
    ];
    Lecture {
        title: "機器學習導論・第 3 堂",
        teacher: "陳老師",
        cues,
        issues,
        glosses,
    }
}
