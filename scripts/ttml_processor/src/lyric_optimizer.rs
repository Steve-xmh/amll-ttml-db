use crate::types::{LyricLine, SyllableSmoothingOptions};

/// 对歌词行应用平滑优化
pub fn apply_smoothing(lines: &mut [LyricLine], options: &SyllableSmoothingOptions) {
    // 因子必须在 (0, 0.5] 范围内。大于0.5可能导致数值不稳定。
    if options.smoothing_iterations == 0 || !(0.0..=0.5).contains(&options.factor) {
        return;
    }

    for line in lines {
        if line.main_syllables.len() < 2 {
            continue;
        }

        let mut start_index = 0;
        while start_index < line.main_syllables.len() {
            // 确定当前组的结束索引
            let next_break = line.main_syllables[start_index..].windows(2).position(|w| {
                let syl_a = &w[0];
                let syl_b = &w[1];
                let duration_a = syl_a.end_ms.saturating_sub(syl_a.start_ms);
                let duration_b = syl_b.end_ms.saturating_sub(syl_b.start_ms);
                let gap = syl_b.start_ms.saturating_sub(syl_a.end_ms);

                duration_a.abs_diff(duration_b) > options.duration_threshold_ms
                    || gap > options.gap_threshold_ms
            });

            let end_index = match next_break {
                Some(break_pos) => start_index + break_pos,
                None => line.main_syllables.len() - 1,
            };

            // 如果有多个音节，就执行平滑处理
            if end_index > start_index {
                let original_start_ms = line.main_syllables[start_index].start_ms;
                let original_end_ms = line.main_syllables[end_index].end_ms;

                let group_slice = &mut line.main_syllables[start_index..=end_index];
                let original_total_duration: f64 = group_slice
                    .iter()
                    .map(|s| s.end_ms.saturating_sub(s.start_ms) as f64)
                    .sum();

                let group_len = group_slice.len();

                let original_gaps: Vec<u64> = group_slice
                    .windows(2)
                    .map(|w| w[1].start_ms.saturating_sub(w[0].end_ms))
                    .collect();

                let mut durations: Vec<f64> = group_slice
                    .iter()
                    .map(|s| s.end_ms.saturating_sub(s.start_ms) as f64)
                    .collect();
                let mut next_durations = vec![0.0; group_len];

                for _ in 0..options.smoothing_iterations {
                    // 处理第一个元素
                    next_durations[0] =
                        (1.0 - options.factor) * durations[0] + options.factor * durations[1];

                    // 处理中间元素
                    for i in 1..group_len - 1 {
                        next_durations[i] = (1.0 - 2.0 * options.factor) * durations[i]
                            + options.factor * durations[i - 1]
                            + options.factor * durations[i + 1];
                    }

                    // 处理最后一个元素
                    let last_idx = group_len - 1;
                    next_durations[last_idx] = (1.0 - options.factor) * durations[last_idx]
                        + options.factor * durations[last_idx - 1];

                    std::mem::swap(&mut durations, &mut next_durations);
                }

                // 重新分配时间戳
                let new_total_duration: f64 = durations.iter().sum();
                if new_total_duration > 1e-6 {
                    let scale_factor = original_total_duration / new_total_duration;
                    durations.iter_mut().for_each(|d| *d *= scale_factor);
                }

                let mut current_ms = original_start_ms;
                for i in 0..group_slice.len() {
                    group_slice[i].start_ms = current_ms;
                    let new_duration = durations[i].round() as u64;
                    group_slice[i].end_ms = current_ms.saturating_add(new_duration);

                    if let Some(gap) = original_gaps.get(i) {
                        current_ms = group_slice[i].end_ms.saturating_add(*gap);
                    }
                }

                // 校准一下最后的时间戳
                if let Some(last_syl_mut) = group_slice.last_mut() {
                    last_syl_mut.end_ms = original_end_ms;
                }
            }

            // 从下一组继续
            start_index = end_index + 1;
        }
    }
}
