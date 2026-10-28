// Command onley-app is onley's desktop client.
package main

import (
	"runtime"

	"github.com/oneo-me/nsgo"

	"onley/internal/appdir"
	"onley/internal/db"
)

func main() {
	cfg := Config{Workers: max(1, runtime.NumCPU()-1)}
	path, err := appdir.DBPath()
	if err != nil {
		cfg.OpenErr = err
	} else {
		cfg.DBPath = path
		store, err := db.Open(path)
		if err != nil {
			cfg.OpenErr = err
		} else {
			cfg.Store = store
		}
	}
	nsgo.Run(Root(cfg))
}
