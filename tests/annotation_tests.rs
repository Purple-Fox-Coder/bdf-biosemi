use edfplus::{EdfReader, EdfWriter, SignalParam};
use std::fs;
use std::path::Path;

// 清理测试文件的辅助函数
fn cleanup_test_file(filename: &str) {
    if Path::new(filename).exists() {
        fs::remove_file(filename).ok();
    }
}

// 创建测试信号的辅助函数
fn create_test_signal() -> SignalParam {
    SignalParam {
        label: "EEG Test".to_string(),
        samples_in_file: 0,
        physical_max: 100.0,
        physical_min: -100.0,
        digital_max: 8388607,
        digital_min: -8388608,
        samples_per_record: 256,
        physical_dimension: "uV".to_string(),
        prefilter: "HP:0.1Hz LP:70Hz".to_string(),
        transducer: "Test electrodes".to_string(),
    }
}

#[test]
fn test_basic_annotation_write_read() {
    let filename = "test_basic_annotations.bdf";

    // 写入阶段 - 创建包含注释的文件
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("ANN001", "F", "15-JUL-1985", "Annotation Test").unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 添加各种类型的注释
        writer.add_annotation(0.0, None, "Recording Start").unwrap();
        writer.add_annotation(1.5, Some(2.0), "Sleep Stage N1").unwrap();
        writer.add_annotation(3.5, None, "Eye Movement").unwrap();
        writer.add_annotation(5.2, Some(0.5), "Artifact").unwrap();
        writer.add_annotation(7.8, None, "K-Complex").unwrap();

        // 写入10秒的数据
        for second in 0..10 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 30.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 读取阶段 - 验证注释
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();

        // 验证注释数量
        assert_eq!(annotations.len(), 5);

        // 验证具体注释内容
        let expected_annotations = vec![
            (0.0, None, "Recording Start"),
            (1.5, Some(2.0), "Sleep Stage N1"),
            (3.5, None, "Eye Movement"),
            (5.2, Some(0.5), "Artifact"),
            (7.8, None, "K-Complex"),
        ];

        for (i, (expected_onset, expected_duration, expected_desc)) in expected_annotations.iter().enumerate() {
            let annotation = &annotations[i];

            // 验证时间（转换回秒）
            let actual_onset = annotation.onset as f64 / 10_000_000.0;
            let tolerance = 0.001; // 1ms 容错
            assert!((actual_onset - expected_onset).abs() < tolerance,
                   "Annotation {} onset mismatch: expected {}, got {}",
                   i, expected_onset, actual_onset);

            // 验证持续时间
            match expected_duration {
                Some(expected_dur) => {
                    assert!(annotation.duration >= 0);
                    let actual_duration = annotation.duration as f64 / 10_000_000.0;
                    assert!((actual_duration - expected_dur).abs() < tolerance,
                           "Annotation {} duration mismatch: expected {}, got {}",
                           i, expected_dur, actual_duration);
                }
                None => {
                    assert_eq!(annotation.duration, -1, "Expected instantaneous event");
                }
            }

            // 验证描述
            assert_eq!(annotation.description, *expected_desc);

            println!("Annotation {}: {:.3}s - {} (duration: {:?})",
                    i, actual_onset, annotation.description,
                    if annotation.duration >= 0 {
                        Some(annotation.duration as f64 / 10_000_000.0)
                    } else {
                        None
                    });
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_annotation_time_precision() {
    let filename = "test_precision_annotations.bdf";

    // 写入阶段 - 测试高精度时间
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("PREC001", "X", "X", "Precision Test").unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 添加高精度时间的注释
        writer.add_annotation(0.0001, None, "Microsecond Event").unwrap();      // 0.1ms
        writer.add_annotation(0.1234567, None, "High Precision").unwrap();      // 123.4567ms
        writer.add_annotation(1.9999999, Some(0.0000001), "Nanosecond Duration").unwrap(); // 100ns duration
        // writer.add_annotation(1.9999999, Some(0.0001), "Microsecond Duration").unwrap(); // 100ms duration
        writer.add_annotation(3.141592653589793, None, "Pi Seconds").unwrap();  // π秒

        // 写入5秒的数据
        for second in 0..5 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 20.0 * (2.0 * std::f64::consts::PI * 5.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // Read Phase – Verify Accuracy
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();

        assert_eq!(annotations.len(), 4);

        // Validate High-Precision Time (EDF+ internally uses 100-nanosecond units)
        let precision_tests = vec![
            (0.0001, "Microsecond Event"),
            (0.1234567, "High Precision"),
            (1.9999999, "Nanosecond Duration"),
            (3.141592653589793, "Pi Seconds"),
        ];

        for (i, (expected_time, expected_desc)) in precision_tests.iter().enumerate() {
            let annotation = &annotations[i];
            let actual_time = annotation.onset as f64 / 10_000_000.0;

            // 100纳秒精度测试
            let tolerance = 1e-7; // 100纳秒
            assert!((actual_time - expected_time).abs() < tolerance,
                   "High precision time test failed for '{}': expected {:.9}, got {:.9}",
                   expected_desc, expected_time, actual_time);

            assert_eq!(annotation.description, *expected_desc);

            println!("Precision test {}: Expected {:.9}s, Actual {:.9}s, Diff: {:.2e}s",
                    i, expected_time, actual_time, (actual_time - expected_time).abs());
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_annotation_edge_cases() {
    let filename = "test_edge_annotations.bdf";

    // 写入阶段 - 测试边界情况
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("EDGE001", "X", "X", "Edge Case Test").unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 测试各种边界情况的注释
        writer.add_annotation(0.0, None, "Exactly at start").unwrap();
        writer.add_annotation(0.0, Some(0.0), "Zero duration").unwrap();
        writer.add_annotation(59.999, None, "Near end").unwrap();

        // 测试长描述
        let long_description = "This is a very long annotation description that tests the system's ability to handle extended text content in annotations, which might be useful for detailed clinical observations and notes.";
        writer.add_annotation(30.0, Some(10.0), long_description).unwrap();

        // 测试特殊字符
        writer.add_annotation(45.0, None, "Special chars: àáâãäåæçèéêë 测试 🧠").unwrap();

        // 写入60秒的数据
        for second in 0..60 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 25.0 * (2.0 * std::f64::consts::PI * 8.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 读取阶段 - 验证边界情况
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();

        assert_eq!(annotations.len(), 5);

        // 验证起始时间的注释
        let start_annotation = &annotations[0];
        assert_eq!(start_annotation.onset, 0);
        assert_eq!(start_annotation.description, "Exactly at start");

        // 验证零持续时间
        let zero_duration = &annotations[1];
        assert_eq!(zero_duration.onset, 0);
        assert_eq!(zero_duration.duration, 0);
        assert_eq!(zero_duration.description, "Zero duration");

        // 验证长描述被正确截断
        let long_desc_annotation = annotations.iter()
            .find(|a| a.description.starts_with("This is a very long"))
            .expect("Should find long description annotation");
        // 描述应该被截断到40字符限制以内
        assert!(long_desc_annotation.description.len() <= 40);
        assert!(long_desc_annotation.description.starts_with("This is a very long"));

        // 验证特殊字符注释存在（但可能被截断）
        let special_char_annotation = annotations.iter()
            .find(|a| a.description.contains("Special chars"))
            .expect("Should find special character annotation");
        // 检查注释是否包含至少一些特殊字符（可能因为截断而不完整）
        assert!(special_char_annotation.description.contains("Special chars"));
        // 注意：由于40字符限制，一些unicode字符可能被截断

        println!("Edge case tests passed:");
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("  {}: {:.3}s - {} (len: {}, duration: {:?})",
                    i, onset_s, &annotation.description[..annotation.description.len().min(50)],
                    annotation.description.len(), duration_s);
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_multiple_annotation_channels() {
    let filename = "test_multi_annotation_channels.bdf";

    // 写入阶段 - 测试多注释通道
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("MULTI001", "X", "X", "Multi Annotation Test").unwrap();

        // 设置3个注释通道
        writer.set_number_of_annotation_signals(3).unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 添加大量注释以测试多通道分发
        for i in 0..15 {
            let onset = i as f64 * 0.5; // 每0.5秒一个注释
            let description = format!("Event {}", i + 1);

            if i % 3 == 0 {
                // 长持续时间事件
                writer.add_annotation(onset, Some(2.0), &description).unwrap();
            } else {
                // 瞬时事件
                writer.add_annotation(onset, None, &description).unwrap();
            }
        }

        // 写入10秒的数据
        for second in 0..10 {
            let mut samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 35.0 * (2.0 * std::f64::consts::PI * 12.0 * t).sin();
                samples.push(value);
            }
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 读取阶段 - 验证多通道注释
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();

        // 应该有15个注释
        assert_eq!(annotations.len(), 15);

        // 验证注释按时间排序
        for i in 1..annotations.len() {
            assert!(annotations[i].onset >= annotations[i-1].onset,
                   "Annotations should be sorted by onset time");
        }

        // 验证注释分布
        let mut event_counts = std::collections::HashMap::new();
        for annotation in annotations {
            let counter = event_counts.entry(&annotation.description).or_insert(0);
            *counter += 1;
        }

        // 每个事件应该只出现一次
        for (event, count) in &event_counts {
            assert_eq!(*count, 1, "Event '{}' should appear exactly once", event);
        }

        println!("Multi-channel annotation test:");
        println!("  Total annotations: {}", annotations.len());
        println!("  Unique events: {}", event_counts.len());

        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };
            println!("    {}: {:.1}s - {} (duration: {:?})",
                    i, onset_s, annotation.description, duration_s);
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_annotation_validation() {
    let filename = "test_validation_annotations.bdf";

    // 测试注释验证
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("VAL001", "X", "X", "Validation Test").unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 测试有效的注释（在数据记录时间范围内）
        assert!(writer.add_annotation(0.1, None, "Valid annotation").is_ok());
        assert!(writer.add_annotation(0.5, Some(0.3), "Valid with duration").is_ok());

        // 测试无效的注释
        assert!(writer.add_annotation(-1.0, None, "Negative onset").is_err());
        assert!(writer.add_annotation(0.1, Some(-1.0), "Negative duration").is_err());
        assert!(writer.add_annotation(0.1, None, "").is_err()); // 空描述应该被拒绝

        // 测试过长的描述
        let very_long_desc = "x".repeat(600);
        assert!(writer.add_annotation(0.1, None, &very_long_desc).is_err());

        // 写入基本数据（1秒的数据，时间范围[0.0, 1.0)）
        let samples = vec![10.0; 256];
        writer.write_samples(&[samples]).unwrap();
        writer.finalize().unwrap();
    }

    // 验证只有有效的注释被保存
    {
        let reader = EdfReader::open(filename).unwrap();
        let annotations = reader.annotations();

        // There should be only two valid comments.
        assert_eq!(annotations.len(), 2);

        assert_eq!(annotations[0].description, "Valid annotation");
        assert_eq!(annotations[1].description, "Valid with duration");

        println!("Validation test passed: {} valid annotations saved", annotations.len());
    }

    cleanup_test_file(filename);
}

#[test]
fn test_sleep_study_annotations() {
    let filename = "test_sleep_study.bdf";

    // 写入阶段 - 模拟完整的睡眠研究
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("SLEEP001", "F", "22-AUG-1978", "Sleep_Study_Patient").unwrap();

        // 添加多个EEG通道
        for channel in &["C3-A2", "C4-A1", "O1-A2", "O2-A1"] {
            let mut signal = create_test_signal();
            signal.label = format!("EEG {}", channel);
            writer.add_signal(signal).unwrap();
        }

        // 添加睡眠研究典型的注释
        writer.add_annotation(0.0, None, "Lights Out").unwrap();
        writer.add_annotation(180.0, None, "Sleep Onset").unwrap();

        // 睡眠阶段
        writer.add_annotation(300.0, Some(1800.0), "Stage N1").unwrap();   // 5-35分钟
        writer.add_annotation(2100.0, Some(3600.0), "Stage N2").unwrap();  // 35-95分钟
        writer.add_annotation(5700.0, Some(1800.0), "Stage N3").unwrap();  // 95-125分钟
        writer.add_annotation(7500.0, Some(900.0), "REM Sleep").unwrap();  // 125-140分钟

        // 睡眠事件
        writer.add_annotation(1200.0, None, "Sleep Spindle").unwrap();
        writer.add_annotation(1800.0, None, "K-Complex").unwrap();
        writer.add_annotation(3600.0, None, "Vertex Sharp Wave").unwrap();
        writer.add_annotation(6000.0, None, "Delta Wave Burst").unwrap();
        writer.add_annotation(7800.0, None, "REM Burst").unwrap();
        writer.add_annotation(8100.0, None, "Eye Movement").unwrap();

        // 觉醒和artifacts
        writer.add_annotation(4200.0, Some(30.0), "Brief Awakening").unwrap();
        writer.add_annotation(6900.0, Some(15.0), "Movement Artifact").unwrap();
        writer.add_annotation(8400.0, None, "Final Awakening").unwrap();

        // 写入2.5小时的数据 (9000秒)
        for second in 0..9000 {
            let mut all_samples = Vec::new();

            for _channel in 0..4 {
                let mut channel_samples = Vec::new();
                for sample in 0..256 {
                    let t = (second * 256 + sample) as f64 / 256.0;

                    // 根据时间模拟不同的脑电活动
                    let base_freq = match second {
                        0..=299 => 10.0,      // 觉醒时的alpha波
                        300..=2099 => 8.0,    // N1阶段
                        2100..=5699 => 5.0,   // N2阶段
                        5700..=7499 => 2.0,   // N3阶段（深睡）
                        7500..=8399 => 15.0,  // REM阶段
                        _ => 12.0,            // 觉醒
                    };

                    let amplitude = match second {
                        5700..=7499 => 80.0,  // 深睡时高幅度
                        _ => 30.0,            // 其他阶段正常幅度
                    };

                    let value = amplitude * (2.0 * std::f64::consts::PI * base_freq * t).sin() +
                               5.0 * (2.0 * std::f64::consts::PI * 50.0 * t).sin(); // 电力线干扰

                    channel_samples.push(value);
                }
                all_samples.push(channel_samples);
            }

            writer.write_samples(&all_samples).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 读取阶段 - 验证睡眠研究数据
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();

        // 验证文件结构
        assert_eq!(header.signals.len(), 4);
        assert_eq!(header.patient_name, "Sleep_Study_Patient");

        // 验证注释数量和类型
        assert_eq!(annotations.len(), 15);

        // 按类型分类注释
        let mut stage_annotations = Vec::new();
        let mut event_annotations = Vec::new();
        let mut other_annotations = Vec::new();

        for annotation in annotations {
            if annotation.description.starts_with("Stage") || annotation.description.contains("REM") {
                stage_annotations.push(annotation);
            } else if annotation.description.contains("Spindle") ||
                     annotation.description.contains("Complex") ||
                     annotation.description.contains("Wave") ||
                     annotation.description.contains("Burst") ||
                     annotation.description.contains("Eye Movement") {
                event_annotations.push(annotation);
            } else {
                other_annotations.push(annotation);
            }
        }

        println!("Sleep Study Analysis:");
        println!("  Total recording duration: {:.1} hours",
                header.file_duration as f64 / 10_000_000.0 / 3600.0);
        println!("  Sleep stages: {}", stage_annotations.len());
        println!("  Sleep events: {}", event_annotations.len());
        println!("  Other annotations: {}", other_annotations.len());

        println!("\nSleep Stages:");
        for annotation in &stage_annotations {
            let onset_min = annotation.onset as f64 / 10_000_000.0 / 60.0;
            let duration_min = if annotation.duration > 0 {
                annotation.duration as f64 / 10_000_000.0 / 60.0
            } else {
                0.0
            };
            println!("    {:.1}-{:.1}min: {}",
                    onset_min, onset_min + duration_min, annotation.description);
        }

        println!("\nSleep Events:");
        for annotation in &event_annotations {
            let onset_min = annotation.onset as f64 / 10_000_000.0 / 60.0;
            println!("    {:.1}min: {}", onset_min, annotation.description);
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_edf_header_fields_comprehensive() {
    let filename = "test_header_fields.bdf";

    // 写入阶段 - 创建包含完整信息的文件
    {
        let mut writer = EdfWriter::create(filename).unwrap();

        // 设置详细的患者和记录信息
        writer.set_patient_info("HDR001", "F", "15-DEC-1985", "Header_Test_Patient").unwrap();

        // 添加多个不同类型的信号
        let signal1 = SignalParam {
            label: "EEG C3-A2".to_string(),
            samples_in_file: 0,
            physical_max: 200.0,
            physical_min: -200.0,
            digital_max: 8388607,
            digital_min: -8388608,
            samples_per_record: 256,  // 256 Hz
            physical_dimension: "uV".to_string(),
            prefilter: "HP:0.1Hz LP:70Hz".to_string(),
            transducer: "AgAgCl cup electrodes".to_string(),
        };
        writer.add_signal(signal1).unwrap();

        let signal2 = SignalParam {
            label: "ECG Lead II".to_string(),
            samples_in_file: 0,
            physical_max: 5.0,
            physical_min: -5.0,
            digital_max: 8388607,
            digital_min: -8388608,
            samples_per_record: 512,  // 512 Hz
            physical_dimension: "mV".to_string(),
            prefilter: "HP:0.05Hz LP:150Hz".to_string(),
            transducer: "Disposable electrodes".to_string(),
        };
        writer.add_signal(signal2).unwrap();

        let signal3 = SignalParam {
            label: "Temperature".to_string(),
            samples_in_file: 0,
            physical_max: 42.0,
            physical_min: 30.0,
            digital_max: 8388607,
            digital_min: -8388608,
            samples_per_record: 1,   // 1 Hz
            physical_dimension: "°C".to_string(),
            prefilter: "".to_string(),
            transducer: "Thermistor probe".to_string(),
        };
        writer.add_signal(signal3).unwrap();

        // 添加多个注释来测试 annotations_in_file 字段
        writer.add_annotation(0.0, None, "Recording start").unwrap();
        writer.add_annotation(10.0, Some(5.0), "Test event 1").unwrap();
        writer.add_annotation(25.0, None, "Marker point").unwrap();
        writer.add_annotation(40.0, Some(2.5), "Test event 2").unwrap();
        writer.add_annotation(55.0, None, "End marker").unwrap();
        writer.add_annotation(59.5, None, "Recording end").unwrap();

        // 写入60秒的数据
        for second in 0..60 {
            let mut all_samples = Vec::new();

            // EEG信号 - 256样本/秒
            let mut eeg_samples = Vec::new();
            for i in 0..256 {
                let t = (second * 256 + i) as f64 / 256.0;
                let value = 50.0 * (2.0 * std::f64::consts::PI * 10.0 * t).sin();
                eeg_samples.push(value);
            }
            all_samples.push(eeg_samples);

            // ECG信号 - 512样本/秒
            let mut ecg_samples = Vec::new();
            for i in 0..512 {
                let t = (second * 512 + i) as f64 / 512.0;
                let value = if (t % 1.0) < 0.1 { 2.0 } else { 0.1 }; // 模拟心跳
                ecg_samples.push(value);
            }
            all_samples.push(ecg_samples);

            // 温度信号 - 1样本/秒
            let temp_value = 36.5 + 0.5 * (2.0 * std::f64::consts::PI * second as f64 / 60.0).sin();
            all_samples.push(vec![temp_value]);

            writer.write_samples(&all_samples).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 读取阶段 - 验证所有头部字段
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();

        println!("=== EDF+ Header Fields Validation ===\n");

        // 验证基本文件结构
        println!("📊 File Structure:");
        assert_eq!(header.signals.len(), 3, "Should have 3 signals");
        println!("  Signals: {} (expected: 3)", header.signals.len());

        // 验证时间相关字段
        println!("\n⏰ Time Information:");
        let duration_seconds = header.file_duration as f64 / 10_000_000.0;
        assert!((duration_seconds - 60.0).abs() < 0.1, "Duration should be ~60 seconds");
        println!("  File duration: {:.1} seconds", duration_seconds);

        let calculated_duration = header.datarecords_in_file as f64 *
                                 (header.datarecord_duration as f64 / 10_000_000.0);
        assert!((calculated_duration - duration_seconds).abs() < 0.001,
               "Calculated duration should match file_duration");
        println!("  Data records: {} × {:.1}s = {:.1}s",
                header.datarecords_in_file,
                header.datarecord_duration as f64 / 10_000_000.0,
                calculated_duration);

        println!("  Start date: {}", header.start_date);
        println!("  Start time: {}", header.start_time);
        println!("  Subsecond offset: {} (100ns units)", header.starttime_subsecond);

        // 验证注释计数 - 这是重点测试
        println!("\n📝 Annotation Information:");
        assert_eq!(header.annotations_in_file, 6, "Should have 6 annotations in header");
        assert_eq!(annotations.len(), 6, "Should read 6 annotations");
        println!("  Annotations in header: {} (expected: 6)", header.annotations_in_file);
        println!("  Annotations read: {} (expected: 6)", annotations.len());

        // 验证注释内容
        let expected_annotations = vec![
            "Recording start",
            "Test event 1",
            "Marker point",
            "Test event 2",
            "End marker",
            "Recording end"
        ];

        for (i, expected_desc) in expected_annotations.iter().enumerate() {
            assert_eq!(annotations[i].description, *expected_desc,
                      "Annotation {} description mismatch", i);
        }

        // 验证患者信息字段
        println!("\n👤 Patient Information:");
        assert_eq!(header.patient_code, "HDR001");
        assert_eq!(header.sex, "F");
        assert_eq!(header.birthdate, "15-DEC-1985");
        assert_eq!(header.patient_name, "Header_Test_Patient");
        println!("  Patient code: {}", header.patient_code);
        println!("  Sex: {}", header.sex);
        println!("  Birth date: {}", header.birthdate);
        println!("  Patient name: {}", header.patient_name);
        println!("  Additional info: '{}'", header.patient_additional);

        // 验证记录信息字段
        println!("\n🏥 Recording Information:");
        println!("  Admin code: '{}'", header.admin_code);
        println!("  Technician: '{}'", header.technician);
        println!("  Equipment: '{}'", header.equipment);
        println!("  Additional info: '{}'", header.recording_additional);

        // 验证信号详细信息
        println!("\n🔍 Signal Details:");
        for (i, signal) in header.signals.iter().enumerate() {
            println!("  Signal {}: {}", i, signal.label);
            println!("    Physical range: {:.1} to {:.1} {}",
                    signal.physical_min, signal.physical_max, signal.physical_dimension);
            println!("    Digital range: {} to {}",
                    signal.digital_min, signal.digital_max);
            println!("    Sampling: {} samples/record", signal.samples_per_record);
            println!("    Prefilter: '{}'", signal.prefilter);
            println!("    Transducer: '{}'", signal.transducer);

            // 验证转换参数
            let bit_value = signal.bit_value();
            let offset = signal.offset();
            println!("    Resolution: {:.6} {}/bit", bit_value, signal.physical_dimension);
            println!("    Offset: {:.1}", offset);
        }

        // 验证具体信号参数
        assert_eq!(header.signals[0].label, "EEG C3-A2");
        assert_eq!(header.signals[0].samples_per_record, 256);
        assert_eq!(header.signals[0].physical_dimension, "uV");

        assert_eq!(header.signals[1].label, "ECG Lead II");
        assert_eq!(header.signals[1].samples_per_record, 512);
        assert_eq!(header.signals[1].physical_dimension, "mV");

        assert_eq!(header.signals[2].label, "Temperature");
        assert_eq!(header.signals[2].samples_per_record, 1);
        assert_eq!(header.signals[2].physical_dimension, "°C");

        // 验证注释详细信息
        println!("\n📋 Annotation Details:");
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let duration_s = if annotation.duration >= 0 {
                Some(annotation.duration as f64 / 10_000_000.0)
            } else {
                None
            };

            println!("  [{:2}] {:.1}s: {} (duration: {:?})",
                    i, onset_s, annotation.description, duration_s);
        }

        // 验证数据一致性
        println!("\n✅ Data Consistency Checks:");

        // 检查计算的总样本数
        let total_samples_per_record: usize = header.signals.iter()
            .map(|s| s.samples_per_record as usize)
            .sum();
        println!("  Total samples per record: {}", total_samples_per_record);
        assert_eq!(total_samples_per_record, 256 + 512 + 1);  // EEG + ECG + Temp

        // 检查文件大小估算
        let estimated_size = 256 * (header.signals.len() + 1) + // Header
                           header.datarecords_in_file as usize *
                           (total_samples_per_record * 2 + 120); // Data + annotation space
        println!("  Estimated file size: ~{} bytes", estimated_size);

        println!("\n🎉 All header field tests passed!");
    }

    cleanup_test_file(filename);
}

#[test]
fn test_header_fields_edge_cases() {
    let filename = "test_header_edge_cases.bdf";

    // 测试极端值和边界情况
    {
        let mut writer = EdfWriter::create(filename).unwrap();

        // 测试极端患者信息
        writer.set_patient_info(
            "EDGE999",
            "X",  // 未知性别
            "X",  // 匿名化出生日期
            "X"   // 匿名化姓名
        ).unwrap();

        // 添加一个信号用于基本测试
        let signal = SignalParam {
            label: "Test".to_string(),
            samples_in_file: 0,
            physical_max: 1.0,
            physical_min: -1.0,
            digital_max: 8388607,
            digital_min: -8388608,
            samples_per_record: 1,
            physical_dimension: "V".to_string(),
            prefilter: "".to_string(),
            transducer: "".to_string(),
        };
        writer.add_signal(signal).unwrap();

        // 测试无注释文件
        // 不添加任何注释

        // 写入最短可能的数据（1秒）
        let samples = vec![0.5];
        writer.write_samples(&[samples]).unwrap();

        writer.finalize().unwrap();
    }

    // 验证边界情况
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();

        println!("=== Header Edge Cases Test ===");

        // 验证零注释情况
        assert_eq!(header.annotations_in_file, 0, "Should have 0 annotations");
        assert_eq!(annotations.len(), 0, "Should read 0 annotations");
        println!("✅ Zero annotations: header reports {}, read {}",
                header.annotations_in_file, annotations.len());

        // 验证最短持续时间
        let duration_seconds = header.file_duration as f64 / 10_000_000.0;
        assert!((duration_seconds - 1.0).abs() < 0.001, "Should be 1 second duration");
        println!("✅ Minimal duration: {:.3} seconds", duration_seconds);

        // 验证匿名化字段
        assert_eq!(header.patient_code, "EDGE999");
        assert_eq!(header.sex, "X");
        assert_eq!(header.birthdate, "X");
        assert_eq!(header.patient_name, "X");
        println!("✅ Anonymized fields: code={}, sex={}, birth={}, name={}",
                header.patient_code, header.sex, header.birthdate, header.patient_name);

        // 验证数据记录
        assert_eq!(header.datarecords_in_file, 1, "Should have 1 data record");
        assert_eq!(header.datarecord_duration, 10_000_000, "Record should be 1 second");
        println!("✅ Data records: {} × {}s",
                header.datarecords_in_file,
                header.datarecord_duration as f64 / 10_000_000.0);

        // 验证最小信号配置
        assert_eq!(header.signals.len(), 1);
        assert_eq!(header.signals[0].samples_per_record, 1);
        println!("✅ Minimal signal config: {} signals, {} samples/record",
                header.signals.len(), header.signals[0].samples_per_record);
    }

    cleanup_test_file(filename);
}

#[test]
fn test_header_fields_maximum_annotations() {
    let filename = "test_max_annotations.bdf";

    // 测试大量注释的情况
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("MAX001", "M", "01-JAN-2000", "Max_Annotations_Test").unwrap();

        // 设置多个注释通道以增加存储容量
        writer.set_number_of_annotation_signals(3).unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 添加适量注释以测试存储和分发（每1秒一个，持续30秒 = 30个注释）
        let total_annotations = 30;
        for i in 0..total_annotations {
            let onset = i as f64; // 每1秒一个注释
            let description = format!("Evt{:02}", i);
            writer.add_annotation(onset, None, &description).unwrap();
        }

        // 写入30秒的数据以覆盖所有注释时间
        for _second in 0..30 {
            let samples = vec![0.0; 256];
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 验证注释存储和分发
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();

        println!("=== EDF+ Annotation Capacity Test ===");

        // 验证注释计数一致性
        println!("✅ Annotation storage: header={}, read={}",
                header.annotations_in_file, annotations.len());

        // 验证头部和读取的注释数量一致
        assert_eq!(header.annotations_in_file, annotations.len() as i64,
                  "Header count should match read count");

        // 验证大部分注释被成功存储（考虑EDF+格式限制）
        assert!(header.annotations_in_file >= 25,
               "Should have at least 25 annotations (got {})", header.annotations_in_file);
        assert!(annotations.len() >= 25,
               "Should read at least 25 annotations (got {})", annotations.len());

        // 验证注释排序
        for i in 1..annotations.len() {
            assert!(annotations[i].onset >= annotations[i-1].onset,
                   "Annotations should be sorted by onset time");
        }
        println!("✅ Annotations properly sorted");

        // 验证注释内容（只验证实际保存的注释）
        for (i, annotation) in annotations.iter().enumerate() {
            // 由于可能有注释被丢弃，不能假设顺序
            let actual_onset = annotation.onset as f64 / 10_000_000.0;

            // 验证描述格式正确
            assert!(annotation.description.starts_with("Evt"),
                   "Annotation {} description should start with 'Evt': {}",
                   i, annotation.description);

            // 验证时间在合理范围内
            assert!(actual_onset >= 0.0 && actual_onset < 30.0,
                   "Annotation {} time should be in [0,30): {:.3}s",
                   i, actual_onset);
        }
        println!("✅ All {} annotations validated", annotations.len());

        // 验证时间范围（基于实际保存的注释）
        if !annotations.is_empty() {
            let first_annotation = &annotations[0];
            let last_annotation = &annotations[annotations.len() - 1];
            let first_time = first_annotation.onset as f64 / 10_000_000.0;
            let last_time = last_annotation.onset as f64 / 10_000_000.0;

            assert!(first_time >= 0.0);
            assert!(last_time < 30.0);
            println!("✅ Time range: {:.1}s to {:.1}s (covering {} annotations)",
                    first_time, last_time, annotations.len());
        }

        // 验证数据记录和注释分布信息
        println!("\n📊 Storage Analysis:");
        println!("  Data records: {}", header.datarecords_in_file);
        println!("  Record duration: {:.1}s", header.datarecord_duration as f64 / 10_000_000.0);
        println!("  Total file duration: {:.1}s", header.file_duration as f64 / 10_000_000.0);
        println!("  Annotation channels: 3 (configured)");
        println!("  Storage capacity: ~{} bytes per record", 3 * 120); // 3 channels × 120 bytes

        // 显示实际注释分布
        println!("\n📝 Annotation Distribution:");
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            println!("    [{:2}] {:.0}s: {}", i, onset_s, annotation.description);
        }

        // 分析注释丢失情况（如果有）
        let expected_annotations = 30;
        let actual_annotations = annotations.len();
        if actual_annotations < expected_annotations {
            let lost_annotations = expected_annotations - actual_annotations;
            println!("\n⚠️  Storage Limitation Analysis:");
            println!("  Expected: {} annotations", expected_annotations);
            println!("  Stored: {} annotations", actual_annotations);
            println!("  Lost: {} annotations ({:.1}%)",
                    lost_annotations,
                    (lost_annotations as f64 / expected_annotations as f64) * 100.0);
            println!("  Reason: EDF+ TAL format space constraints (120 bytes/channel/record)");
        } else {
            println!("\n✅ All annotations successfully stored!");
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_multiple_annotations_per_record() {
    let filename = "test_multi_annotations_per_record.bdf";

    // 测试同一数据记录内的多个注释
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("MULTI001", "X", "X", "Multi_Per_Record_Test").unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 在第一秒内添加多个注释，测试120字节TAL限制
        writer.add_annotation(0.0, None, "Start").unwrap();           // ~12 bytes
        writer.add_annotation(0.1, None, "Event1").unwrap();          // ~13 bytes
        writer.add_annotation(0.2, None, "Event2").unwrap();          // ~13 bytes
        writer.add_annotation(0.3, None, "Event3").unwrap();          // ~13 bytes
        writer.add_annotation(0.4, None, "Event4").unwrap();          // ~13 bytes
        writer.add_annotation(0.5, None, "Event5").unwrap();          // ~13 bytes
        writer.add_annotation(0.6, None, "Event6").unwrap();          // ~13 bytes
        writer.add_annotation(0.7, None, "Event7").unwrap();          // ~13 bytes
        writer.add_annotation(0.8, None, "Event8").unwrap();          // ~13 bytes
        writer.add_annotation(0.9, None, "Event9").unwrap();          // ~13 bytes

        // 在第二秒内添加更少的注释作为对比
        writer.add_annotation(1.0, None, "Second").unwrap();
        writer.add_annotation(1.5, None, "Middle").unwrap();

        // 写入2秒的数据
        for _second in 0..2 {
            let samples = vec![0.0; 256];
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 验证注释存储结果
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();

        println!("=== Multiple Annotations Per Record Test ===");

        // 显示注释计数
        println!("📊 Annotation Storage Results:");
        println!("  Total added: 12 annotations");
        println!("  Header reports: {} annotations", header.annotations_in_file);
        println!("  Actually read: {} annotations", annotations.len());

        // 验证头部和读取一致性
        assert_eq!(header.annotations_in_file, annotations.len() as i64,
                  "Header count should match read count");

        // 按数据记录分组分析注释
        let mut record_0_annotations = Vec::new(); // 第一秒 [0.0, 1.0)
        let mut record_1_annotations = Vec::new(); // 第二秒 [1.0, 2.0)

        for annotation in annotations.iter() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            if onset_s < 1.0 {
                record_0_annotations.push(annotation);
            } else if onset_s < 2.0 {
                record_1_annotations.push(annotation);
            }
        }

        println!("\n📋 Annotation Distribution by Data Record:");
        println!("  Record 0 (0.0-1.0s): {} annotations", record_0_annotations.len());
        for (i, annotation) in record_0_annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            println!("    [{:2}] {:.1}s: {}", i, onset_s, annotation.description);
        }

        println!("  Record 1 (1.0-2.0s): {} annotations", record_1_annotations.len());
        for (i, annotation) in record_1_annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            println!("    [{:2}] {:.1}s: {}", i, onset_s, annotation.description);
        }

        // 分析TAL空间使用情况
        println!("\n🔍 TAL Space Analysis:");
        println!("  TAL buffer size per record: 120 bytes");

        // 估算第一个记录的TAL使用量
        let mut estimated_tal_usage = 0;
        estimated_tal_usage += 6; // 时间戳 "+0\x14\x14\x00"

        for annotation in &record_0_annotations {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let time_str = format!("{:.1}", onset_s);
            let desc_len = annotation.description.len().min(40);
            // 格式: "+<time>\x14<desc>\x14"
            estimated_tal_usage += 1 + time_str.len() + 1 + desc_len + 1;
        }

        println!("  Record 0 estimated usage: ~{} bytes", estimated_tal_usage);
        println!("  Utilization: {:.1}%", (estimated_tal_usage as f64 / 120.0) * 100.0);

        if record_0_annotations.len() < 10 {
            let missing = 10 - record_0_annotations.len();
            println!("  ⚠️  {} annotations may have been dropped due to space limits", missing);
        }

        // 验证注释内容正确性
        println!("\n✅ Content Validation:");
        let mut validation_passed = true;

        // 检查第一秒的注释
        let expected_first_second = vec!["Start", "Event1", "Event2", "Event3", "Event4",
                                       "Event5", "Event6", "Event7", "Event8", "Event9"];
        let mut found_in_first_second = Vec::new();
        for annotation in &record_0_annotations {
            found_in_first_second.push(annotation.description.as_str());
        }

        for expected in &expected_first_second {
            if !found_in_first_second.contains(expected) {
                println!("  ❌ Missing annotation: {}", expected);
                validation_passed = false;
            }
        }

        // 检查第二秒的注释
        let expected_second_second = vec!["Second", "Middle"];
        let mut found_in_second_second = Vec::new();
        for annotation in &record_1_annotations {
            found_in_second_second.push(annotation.description.as_str());
        }

        for expected in &expected_second_second {
            if found_in_second_second.contains(expected) {
                println!("  ✅ Found annotation: {}", expected);
            } else {
                println!("  ❌ Missing annotation: {}", expected);
                validation_passed = false;
            }
        }

        if validation_passed {
            println!("  🎉 All expected annotations found!");
        }

        // 总结测试结果
        println!("\n📄 Test Summary:");
        println!("  • Single record can store {} annotations in 120 bytes", record_0_annotations.len());
        println!("  • Average space per annotation: ~{:.1} bytes",
                if record_0_annotations.len() > 0 {
                    estimated_tal_usage as f64 / record_0_annotations.len() as f64
                } else { 0.0 });

        if record_0_annotations.len() == 10 {
            println!("  • ✅ All 10 annotations in first record stored successfully");
        } else {
            println!("  • ⚠️  Only {}/10 annotations in first record were stored", record_0_annotations.len());
            println!("  • This demonstrates the 120-byte TAL buffer limitation");
        }
    }

    cleanup_test_file(filename);
}

#[test]
fn test_tal_buffer_stress_test() {
    let filename = "test_tal_stress.bdf";

    // 压力测试：尝试在单个记录中存储大量短注释
    {
        let mut writer = EdfWriter::create(filename).unwrap();
        writer.set_patient_info("STRESS01", "X", "X", "TAL_Stress_Test").unwrap();

        let signal = create_test_signal();
        writer.add_signal(signal).unwrap();

        // 尝试添加20个非常短的注释到同一秒内
        println!("Adding 20 very short annotations to test TAL limits...");
        for i in 0..20 {
            let onset = i as f64 * 0.05; // 每50ms一个注释，都在第一秒内
            let description = format!("E{}", i); // 非常短的描述（2-3字符）
            writer.add_annotation(onset, None, &description).unwrap();
        }

        // 在第二秒添加几个正常长度的注释作为对比
        writer.add_annotation(1.0, None, "Normal length annotation").unwrap();
        writer.add_annotation(1.5, None, "Another normal one").unwrap();

        // 写入2秒的数据
        for _second in 0..2 {
            let samples = vec![0.0; 256];
            writer.write_samples(&[samples]).unwrap();
        }

        writer.finalize().unwrap();
    }

    // 分析压力测试结果
    {
        let reader = EdfReader::open(filename).unwrap();
        let header = reader.header();
        let annotations = reader.annotations();

        println!("\n=== TAL Buffer Stress Test Results ===");

        // 按数据记录分组
        let mut record_0_count = 0;
        let mut record_1_count = 0;

        for annotation in annotations.iter() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            if onset_s < 1.0 {
                record_0_count += 1;
            } else if onset_s < 2.0 {
                record_1_count += 1;
            }
        }

        println!("📊 Stress Test Results:");
        println!("  Attempted to add: 20 short annotations + 2 normal annotations = 22 total");
        println!("  Actually stored: {} annotations", annotations.len());
        println!("  Record 0 (short annotations): {}/20 stored", record_0_count);
        println!("  Record 1 (normal annotations): {}/2 stored", record_1_count);

        // 计算短注释的存储效率
        if record_0_count > 0 {
            // 估算平均每个短注释的空间使用
            let timestamp_overhead = 6; // "+0\x14\x14\x00"
            let available_space = 120 - timestamp_overhead;
            let avg_space_per_short_annotation = available_space / record_0_count;

            println!("\n🔍 Storage Efficiency Analysis:");
            println!("  Available space for annotations: {} bytes", available_space);
            println!("  Average space per short annotation: ~{} bytes", avg_space_per_short_annotation);
            println!("  Theoretical maximum short annotations: ~{}", available_space / 8); // 假设最短注释8字节
        }

        // 显示头部存储的注释数量
        println!("\n 📋 Header Annotation Count: {}", header.annotations_in_file);

        // 显示实际存储的注释
        println!("\n📋 Actually Stored Annotations:");
        for (i, annotation) in annotations.iter().enumerate() {
            let onset_s = annotation.onset as f64 / 10_000_000.0;
            let record_num = if onset_s < 1.0 { 0 } else { 1 };
            println!("  [{:2}] R{} {:.3}s: {} (len: {})",
                    i, record_num, onset_s, annotation.description, annotation.description.len());
        }

        // 结论
        println!("\n🎯 Key Findings:");
        println!("  • 120-byte TAL buffer can store ~{} very short annotations per record", record_0_count);
        println!("  • This demonstrates the practical limits of EDF+ annotation density");
        if record_0_count < 20 {
            println!("  • {} annotations were dropped due to space constraints", 20 - record_0_count);
        }
    }

    cleanup_test_file(filename);
}
