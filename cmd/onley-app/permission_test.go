package main

import (
	"errors"
	"os"
	"testing"
)

func TestIsPermissionError(t *testing.T) {
	if !isPermissionError(&os.PathError{Op: "remove", Path: "x", Err: os.ErrPermission}) {
		t.Errorf("PathError with ErrPermission was not recognized")
	}
	if !isPermissionError(errors.Join(errors.New("context"), os.ErrPermission)) {
		t.Errorf("joined ErrPermission was not recognized")
	}
	if isPermissionError(errors.New("other")) {
		t.Errorf("plain error was recognized as a permission error")
	}
}
