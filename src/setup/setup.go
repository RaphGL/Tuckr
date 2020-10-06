package setup

import (
	"fmt"
	"github.com/logrusorgru/aurora"
	"os"
	"os/exec"
	"strings"
)

/* Contains the functions that do all the setting up as well as
an array with the os.FileInfo for all files in the current dir */
type SetupHandle struct {
	WorkingDir []os.FileInfo
}

/* Checks the files in the directory and loads them into the struct
The files array is only loaded into the struct if a tuckr.json is present */
func NewSetupHandle() (SetupHandle, error) {
	var handler SetupHandle
	dir, err := os.Open(".")
	if err != nil {
		return handler, err
	}
	files, err := dir.Readdir(-1)
	if err != nil {
		return handler, err
	}
	handler = SetupHandle{files}
	return handler, nil
}

// Runs all scripts that start with a set_ prefix
func (s SetupHandle) RunScripts() error {
	var curr string
	for _, file := range s.WorkingDir {
		curr = file.Name()
		if strings.HasPrefix(curr, "set_") {
			fmt.Println(aurora.Green("Running script:"), curr)
			cmd := exec.Command(os.ExpandEnv("$SHELL"), curr)
			cmd.Stdout = os.Stdout
			cmd.Run()
		}
	}
	return nil
}
