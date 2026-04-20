use edfplus::{EdfWriter, SignalParam, EdfReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== EDF+ 流式写入示例 ===");

    // 1. 创建新的EDF+文件
    let mut writer = EdfWriter::create("streaming_test.bdf")?;

    // 2. 设置患者信息
    writer.set_patient_info("STREAM-001", "M", "01-JAN-1990", "流式测试患者")?;

    // 3. 设置数据记录持续时间为0.5秒（更高的时间分辨率）
    writer.set_datarecord_duration(0.5)?;

    // 4. 添加信号定义
    let eeg_signal = SignalParam {
        label: "EEG C3-A1".to_string(),
        samples_in_file: 0,  // 将被自动计算
        physical_max: 200.0,
        physical_min: -200.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 128,  // 0.5秒 × 256Hz = 128样本
        physical_dimension: "uV".to_string(),
        prefilter: "HP:0.1Hz LP:70Hz".to_string(),
        transducer: "AgAgCl cup electrode".to_string(),
    };

    let emg_signal = SignalParam {
        label: "EMG Left Arm".to_string(),
        samples_in_file: 0,
        physical_max: 1000.0,
        physical_min: -1000.0,
        digital_max: 32767,
        digital_min: -32768,
        samples_per_record: 64,   // 0.5秒 × 128Hz = 64样本
        physical_dimension: "uV".to_string(),
        prefilter: "HP:10Hz LP:500Hz".to_string(),
        transducer: "Surface electrode".to_string(),
    };

    writer.add_signal(eeg_signal)?;
    writer.add_signal(emg_signal)?;

    println!("信号定义已添加：");
    println!("  - EEG C3-A1: 256 Hz (128 samples/0.5s)");
    println!("  - EMG Left Arm: 128 Hz (64 samples/0.5s)");

    // 5. 流式写入数据（模拟实时数据采集）
    let total_records = 20;  // 写入10秒的数据（20个0.5秒记录）

    println!("\n开始流式写入 {} 个数据记录...", total_records);

    for record in 0..total_records {
        println!("写入数据记录 {}/{}", record + 1, total_records);

        // 生成EEG数据（模拟alpha波 + 噪声）
        let mut eeg_samples = Vec::new();
        for i in 0..128 {
            let t = (record as f64 * 0.5) + (i as f64 / 256.0);

            // 10Hz alpha wave + 50Hz noise + random component
            let alpha = 50.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
            let noise = 5.0 * (2.0 * std::f64::consts::PI * 50.0 * t).sin();
            let random = (t * 123.456).fract() * 10.0 - 5.0;

            eeg_samples.push(alpha + noise + random);
        }

        // 生成EMG数据（模拟肌肉活动）
        let mut emg_samples = Vec::new();
        for i in 0..64 {
            let t = (record as f64 * 0.5) + (i as f64 / 128.0);

            // 模拟间歇性肌肉活动
            let burst = if (t * 2.0) as i32 % 3 == 0 {
                200.0 * (2.0 * std::f64::consts::PI * 80.0 * t).sin()
            } else {
                10.0 * (t * 45.67).fract() - 5.0  // 基线噪声
            };

            emg_samples.push(burst);
        }

        // 写入这个数据记录
        writer.write_samples(&[eeg_samples, emg_samples])?;

        // 模拟实时采集的延迟
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    println!("\n完成数据写入，正在finalize文件...");

    // 6. 完成文件写入
    writer.finalize()?;

    println!("✅ 流式写入完成！");

    // 7. 验证写入的文件
    println!("\n=== 验证写入的文件 ===");

    let mut reader = EdfReader::open("streaming_test.bdf")?;
    let header = reader.header();

    println!("文件验证结果：");
    println!("  文件格式: EDF+ (European Data Format Plus)");
    println!("  患者代码: {}", header.patient_code);
    println!("  患者姓名: {}", header.patient_name);
    println!("  设备信息: {}", header.equipment);
    println!("  开始时间: {} {}", header.start_date, header.start_time);
    println!("  数据记录数: {}", header.datarecords_in_file);
    println!("  数据记录持续时间: {:.1} 秒", header.datarecord_duration as f64 / 10_000_000.0);
    println!("  信号数量: {}", header.signals.len());

    for (i, signal) in header.signals.iter().enumerate() {
        let sampling_rate = signal.samples_per_record as f64 / (header.datarecord_duration as f64 / 10_000_000.0);
        println!("  信号 {}: {} ({:.1} Hz, {} 样本/记录)",
                i + 1, signal.label,
                sampling_rate,
                signal.samples_per_record);
    }

    println!("  文件持续时间: {:.1} 秒", header.file_duration as f64 / 10_000_000.0);

    // 8. 读取一些样本进行验证
    println!("\n=== 读取样本验证 ===");

    let eeg_samples = reader.read_physical_samples(0, 100)?;
    let emg_samples = reader.read_physical_samples(1, 50)?;

    println!("EEG 前10个样本: {:?}", &eeg_samples[..10.min(eeg_samples.len())]);
    println!("EMG 前10个样本: {:?}", &emg_samples[..10.min(emg_samples.len())]);

    // 清理测试文件
    std::fs::remove_file("streaming_test.bdf").ok();

    println!("\n🎉 流式写入测试完成！所有功能正常工作。");

    Ok(())
}
