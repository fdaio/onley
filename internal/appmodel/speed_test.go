package appmodel

import (
	"testing"
	"time"
)

func TestRate(t *testing.T) {
	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	samples := []SpeedSample{
		{At: now.Add(-1500 * time.Millisecond), Bytes: 100},
		{At: now.Add(-500 * time.Millisecond), Bytes: 200},
		{At: now.Add(-100 * time.Millisecond), Bytes: 300},
	}
	if got := Rate(samples, now, time.Second); got != 500 {
		t.Errorf("Rate = %v, want 500", got)
	}
	if got := Rate(nil, now, time.Second); got != 0 {
		t.Errorf("empty Rate = %v, want 0", got)
	}
	if got := Rate(samples, now, 0); got != 0 {
		t.Errorf("zero-window Rate = %v, want 0", got)
	}
}

func TestGaugeMax(t *testing.T) {
	tests := []struct {
		current, speed, want float64
	}{
		{40, 0, 40},
		{40, 30, 40},
		{40, 40, 80},
		{80, 70, 200},
		{200, 150, 200},
		{400, 350, 1000},
		{400, 360, 1000},
		{1000, 5000, 8000},
		{8000, 1, 8000},
		{0, 1, 40},
	}
	for _, tt := range tests {
		if got := GaugeMax(tt.current, tt.speed); got != tt.want {
			t.Errorf("GaugeMax(%v, %v) = %v, want %v", tt.current, tt.speed, got, tt.want)
		}
	}
}
