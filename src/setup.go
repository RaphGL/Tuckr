package main

import (
	"encoding/json"
	"fmt"
	"os"
    "os/exec"
	"strings"
)

type Config struct {
    General struct {
        CloneDotfilesCmd string `json:"cloneDotfilesCmd"`
        DotfilesRepo string `json:"dotfilesRepo"`
        DotfilesDest string `json:"dotfilesDest"`
    }`json:"general"`
    Packages struct {
        PipLocal string `json:"pipLocal"`
        PipGlobal string `json:"pipGlobal"`
        NpmLocal string `json:"npmLocal"`
        NpmGlobal string `json:"npmGlobal"`
        YarnLocal string `json:"yarnLocal"`
        YarnGlocal string `json:"yarnGlobal"`
    }
    Scripts string
}

var config, _ = LoadConfig()

func main() {
    CloneFiles()
    //fmt.Printf("%+v", config)
}

// Load config file to Config struct
func LoadConfig() (Config, error) {
    var config Config
    configFile, err := os.Open("./tuckr.conf")
    defer configFile.Close()
    if err != nil {
        configFile, err = os.Open(os.ExpandEnv("$HOME/.config/tuckr.conf"))
        if err != nil {
            fmt.Println("Error: Could not find config file.")
        }
        return config, err
    }
    jsonParser := json.NewDecoder(configFile)
    jsonParser.Decode(&config)
    return config, err
}

//Clone repos necessary for the dotfiles
func CloneFiles() {
    // runs a custom clone command if CloneDotfilesCmd is set
    if config.General.CloneDotfilesCmd != "" {
        cmdArray := strings.Split(config.General.CloneDotfilesCmd, " ")
        cmdArgs := strings.Join(cmdArray[1:], " ")
        cmd := exec.Command(cmdArray[0], cmdArgs)
        cmd.Stdout = os.Stdout
        cmd.Stderr = os.Stderr
        cmd.Run()
    // if no CloneDotfilesCmd is provide git is used and the dest and src variables read from config file
    } else if config.General.DotfilesDest != "" && config.General.DotfilesRepo != "" {
        cmd := exec.Command("git", "clone", os.ExpandEnv(config.General.DotfilesRepo), os.ExpandEnv(config.General.DotfilesDest))
        cmd.Stdout = os.Stdout
        cmd.Stderr = os.Stderr
        cmd.Run()
    }
}
