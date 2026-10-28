package appmodel

import "time"

// SpeedSample is bytes read at a time.
type SpeedSample struct {
	At    time.Time
	Bytes int64
}

// Rate returns the bytes per second read in the window ending at now.
func Rate(samples []SpeedSample, now time.Time, window time.Duration) float64 {
	if window <= 0 || len(samples) == 0 {
		return 0
	}
	cutoff := now.Add(-window)
	var total int64
	for _, sample := range samples {
		if !sample.At.Before(cutoff) {
			total += sample.Bytes
		}
	}
	return float64(total) / window.Seconds()
}

var gaugeSteps = []float64{40, 80, 200, 400, 1000, 2000, 4000, 8000}

// GaugeMax returns the smallest gauge ceiling at or above current that leaves
// room for speed, or the largest ceiling for a faster scan. It never lowers a
// ceiling.
func GaugeMax(current, speed float64) float64 {
	if current < gaugeSteps[0] {
		current = gaugeSteps[0]
	}
	for _, step := range gaugeSteps {
		if step < current {
			continue
		}
		if speed <= step*0.85 {
			return step
		}
	}
	return gaugeSteps[len(gaugeSteps)-1]
}
