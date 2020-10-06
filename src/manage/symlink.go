package manage

import (
	"errors"
	"fmt"
	"github.com/logrusorgru/aurora"
	"io/ioutil"
	"os"
)

/* Returns files that are symlinked or not
b = true returns symlinks
b = false returns non symlinks */
func GetSymlinks(b bool) ([]os.FileInfo, error) {
	var symlinks []os.FileInfo
	dir, err := ioutil.ReadDir(".")
	if err != nil {
		return symlinks, err
	}
	/* Loop over all files in the directory and check if its a symlink by trying to read
	   the destination of the symlink, if there's no error it's a symlink if there's an error
	   then it's not a symlink */
	for _, f := range dir {
		_, err := os.Readlink(f.Name())
		if b {
			if err == nil {
				symlinks = append(symlinks, f)
			}
		} else {
			if err != nil {
				symlinks = append(symlinks, f)
			}
		}
	}
	return symlinks, nil
}

// Creates symlink from src to dest returns an error if file is already a symlink
func CreateSymlink(dest string, src string) error {
	_, err := os.Readlink(src)
	if err != nil {
		os.Symlink(src, dest)
		return nil
	}
	return errors.New("Error: File is already a symlink")
}

// Removes symlink from src to dest returns an error if file is not a symlink
func RemoveSymlink(src string) error {
	_, err := os.Readlink(src)
	if err != nil {
		return errors.New("Error: File is not a symlink")
	}
	os.Remove(src)
	return nil
}

/* Reads the current directory and symlinks it's files to the location specified by dest
TODO function breaks if a string doesn't end with / */
func CreateSymlinks(dest string) error {
	dir, err := ioutil.ReadDir(".")
	var currFile string
	if err != nil {
		return err
	}
	currDir, err := os.Getwd()
	if err != nil {
		return err
	}
	for _, f := range dir {
		currFile = f.Name()
		// makes sure that it does not try to symlink a symlink
		_, err := os.Readlink(currFile)
		if err != nil {
			err := os.Symlink(currDir+"/"+currFile, dest+currFile)
			if err != nil {
				fmt.Println(aurora.Red("Skipping:"), currFile, "is already a symlink")
			}
		}
	}
	return nil
}

// Remove all symlinks from current directory
//TODO function breaks if a string doesn't end with / */
func RemoveSymlinks(src string) error {
	dir, err := ioutil.ReadDir(src)
	if err != nil {
		return err
	}
	for _, f := range dir {
		//skips non-symlinks
		currFile := src + f.Name()
		_, err := os.Readlink(currFile)
		if err != nil {
			continue
		}
		os.Remove(currFile)
	}
	return nil
}
