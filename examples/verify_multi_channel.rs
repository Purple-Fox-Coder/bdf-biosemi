use edfplus::{EdfReader, Result};

/// 验证多通道EEG文件的内容和注释
fn main() -> Result<()> {
    println!("🔍 正在验证多通道EEG文件...");

    // 打开刚生成的文件
    let mut reader = EdfReader::open("multi_channel_eeg.bdf")?;

    // 先获取所有需要的头部信息
    let signal_count = reader.header().signals.len();
    let file_duration = reader.header().file_duration;
    let datarecords_count = reader.header().datarecords_in_file;
    let datarecord_duration = reader.header().datarecord_duration;
    let signals_info: Vec<_> = reader.header().signals.iter().map(|s| (
        s.label.clone(),
        s.physical_min,
        s.physical_max,
        s.physical_dimension.clone(),
        s.samples_per_record
    )).collect();

    println!("📋 文件信息:");
    println!("  • 信号数量: {}", signal_count);
    println!("  • 文件时长: {:.1} 秒", file_duration as f64 / 10_000_000.0);
    println!("  • 数据记录数: {}", datarecords_count);
    println!("  • 每记录时长: {:.1} 秒", datarecord_duration as f64 / 10_000_000.0);

    println!("\n📊 信号通道:");
    for (i, (label, phys_min, phys_max, dimension, samples_per_record)) in signals_info.iter().enumerate() {
        println!("  [{}] {} - 范围: {:.1} 到 {:.1} {} ({}样本/记录)",
                 i, label, phys_min, phys_max, dimension, samples_per_record);
    }

    // 检查注释
    println!("\n📝 注释/事件:");
    let annotations = reader.annotations();
    if annotations.is_empty() {
        println!("  ❌ 未找到注释！");
    } else {
        println!("  ✅ 找到 {} 个注释:", annotations.len());
        for annotation in annotations {
            println!("    • {:.1}s: \"{}\"", annotation.onset as f64 / 10_000_000.0, annotation.description);
        }
    }

    // 读取一些样本数据进行验证
    println!("\n🔬 数据样本验证:");
    let num_channels_to_check = 3.min(signal_count);
    for chan_idx in 0..num_channels_to_check {
        let (signal_name, _, _, _, _) = &signals_info[chan_idx];
        let samples = reader.read_physical_samples(chan_idx, 10)?;
        println!("  {} (前10个样本): {:.2?}...", signal_name, &samples[..samples.len().min(3)]);

        // 计算基本统计
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let max = samples.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min = samples.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        println!("    统计: 均值={:.2}, 最小值={:.2}, 最大值={:.2}", mean, min, max);
    }

    println!("\n✅ 多通道EEG文件验证完成！");
    Ok(())
}
