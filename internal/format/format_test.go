package format

import "testing"

func TestBytes(t *testing.T) {
	tests := []struct {
		n    int64
		want string
	}{
		{0, "0 B"},
		{512, "512 B"},
		{1024, "1.0 KB"},
		{2 * 1024 * 1024, "2.0 MB"},
		{3 * 1024 * 1024 * 1024, "3.0 GB"},
	}
	for _, tt := range tests {
		if got := Bytes(tt.n); got != tt.want {
			t.Errorf("Bytes(%d) = %q, want %q", tt.n, got, tt.want)
		}
	}
}
