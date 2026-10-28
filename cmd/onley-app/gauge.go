package main

import (
	"fmt"
	"math"

	"github.com/oneo-me/nsgo/core"
)

// drawGauge draws the scan dial. The needle, sweep and pulse are bound to
// animated values, so the platform moves them without a commit per frame.
func (s *state) drawGauge(c colors) func(*core.Drawing, core.Size) {
	return func(d *core.Drawing, size core.Size) {
		w, h := float32(size.Width), float32(size.Height)
		if w <= 0 || h <= 0 {
			return
		}
		centerX, centerY := w/2, float32(h)*0.58
		radius := min(w/2-18, h*0.44) - 4
		if radius < 12 {
			return
		}
		a0, a1 := float32(0.75*math.Pi), float32(2.25*math.Pi)
		ceiling := s.speedMax.Get()
		if ceiling <= 0 {
			ceiling = 40
		}
		angle := core.Interpolate(s.needle, core.Interpolation{
			Input: []float64{0, 100}, Output: []float64{float64(a0), float64(a1)},
			Extrapolate: core.ExtrapolateClamp,
		})
		const tau = float32(2 * math.Pi)

		// Track and tick ring.
		d.SetLineCap(core.CapRound)
		d.SetLineWidth(10).SetStrokeColor(c.border).BeginPath().
			Arc(centerX, centerY, radius, a0, a1, false).Stroke()
		for i := 0; i <= 40; i++ {
			frac := float64(i) / 40
			a := float64(a0) + frac*float64(a1-a0)
			major := i%5 == 0
			inner := radius - 7
			color := c.border
			width := float32(1)
			if major {
				inner, color, width = radius-12, c.muted, 2
			}
			d.SetStrokeColor(color).SetLineWidth(width).BeginPath().
				MoveTo(centerX+cos32(a)*inner, centerY+sin32(a)*inner).
				LineTo(centerX+cos32(a)*radius, centerY+sin32(a)*radius).Stroke()
			if i%10 == 0 {
				d.SetFillColor(c.muted).SetTextAlign(core.TextAlignCenter).SetTextBaseline(core.BaselineMiddle).
					FillText(fmt.Sprintf("%.0f", ceiling*frac), centerX+cos32(a)*(radius-25), centerY+sin32(a)*(radius-25),
						core.Font{Size: 9, Weight: core.FontWeightNormal})
			}
		}

		// The sweep follows the needle with a muted glow underneath it.
		glow := c.accent.Faded(0.14)
		d.SetLineWidth(14).SetStrokeColor(glow).BeginPath().
			Arc(centerX, centerY, radius, a0, a0, false).Bind(core.ArgA1, angle).Stroke()
		d.SetLineWidth(7).SetStrokeGradient(core.CanvasConic(float32(a0-0.2), centerX, centerY,
			core.GradientStop{Offset: 0, Color: c.accent},
			core.GradientStop{Offset: 0.75, Color: c.accent},
			core.GradientStop{Offset: 1, Color: c.success},
		)).BeginPath().Arc(centerX, centerY, radius, a0, a0, false).Bind(core.ArgA1, angle).Stroke()

		// A pulse keeps the dial moving while no bytes arrive.
		pulseRadius := core.Interpolate(s.pulse, core.Interpolation{
			Input: []float64{0, 1}, Output: []float64{float64(radius * 0.10), float64(radius * 0.40)},
		})
		pulseAlpha := core.Interpolate(s.pulse, core.Interpolation{
			Input: []float64{0, 1}, Output: []float64{0.55, 0},
		})
		d.Save().SetAlpha(0).Bind(core.ArgA, pulseAlpha).SetStrokeColor(c.accent).SetLineWidth(2).
			BeginPath().Arc(centerX, centerY, 0, 0, tau, false).Bind(core.ArgR, pulseRadius).Stroke().Restore()

		// Needle and hub.
		d.Save().Translate(centerX, centerY).Rotate(0).Bind(core.ArgA, angle).
			SetShadow(core.RGBA(0x000000, 0.45), 0, 0, 6).SetFillColor(c.text).
			BeginPath().MoveTo(-12, -3).LineTo(radius-16, 0).LineTo(-12, 3).ClosePath().Fill().Restore()
		d.SetFillColor(c.surface).SetStrokeColor(c.text).SetLineWidth(2).
			BeginPath().Arc(centerX, centerY, 7, 0, tau, false).Fill().Stroke()
		d.SetFillColor(c.accent).BeginPath().Arc(centerX, centerY, 3, 0, tau, false).Fill()
	}
}

func cos32(a float64) float32 { return float32(math.Cos(a)) }
func sin32(a float64) float32 { return float32(math.Sin(a)) }
