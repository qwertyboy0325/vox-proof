#[derive(Debug)]
struct Segment {
    index: u32,
    start_ms: u64,
    end_ms: u64,
    text: String,
}

impl Segment {
    fn duration_ms(&self) -> u64 {
        self.end_ms - self.start_ms
    }

    fn extend_by(&mut self, extra_ms: u64) {
        self.end_ms += extra_ms;
    }
}

fn main() {
    let mut segment = Segment {
        index: 1,
        start_ms: 0,
        end_ms: 2500,
        text: "我們使用 Kafka 處理事件流".to_string(),
    };

    println!(
        "Segment #{} spans {}-{} ms ({} ms) with text \"{}\"",
        segment.index,
        segment.start_ms,
        segment.end_ms,
        segment.duration_ms(),
        segment.text,
    );

    segment.extend_by(500);

    println!(
        "Segment #{} spans {}-{} ms ({} ms) with text \"{}\"",
        segment.index,
        segment.start_ms,
        segment.end_ms,
        segment.duration_ms(),
        segment.text,
    );

    println!("{:#?}", segment);
}