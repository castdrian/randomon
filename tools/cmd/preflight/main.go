package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/castdrian/randomon/tools/internal/repo"
)

type step struct {
	name      string
	directory string
	command   []string
	optional  bool
}

var crossTargets = []string{"x86_64-pc-windows-msvc"}

func main() {
	steps := []step{
		{name: "cargo fmt", command: []string{"cargo", "fmt", "--all", "--check"}},
		{name: "cargo clippy", command: []string{"cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"}},
	}
	for _, target := range crossTargets {
		steps = append(steps, step{
			name:     "cargo clippy for " + target,
			command:  []string{"cargo", "clippy", "--workspace", "--all-targets", "--target", target, "--", "-D", "warnings"},
			optional: true,
		})
	}
	steps = append(steps,
		step{name: "cargo test", command: []string{"cargo", "test", "--workspace"}},
		step{name: "cargo deny", command: []string{"cargo", "deny", "check"}, optional: true},
		step{name: "go vet", directory: repo.Path("tools"), command: []string{"go", "vet", "./..."}},
		step{name: "actionlint", command: []string{"actionlint"}, optional: true},
	)
	var skipped []string
	for _, current := range steps {
		if current.optional && !available(current) {
			skipped = append(skipped, current.name)
			continue
		}
		fmt.Printf("==> %s\n", current.name)
		directory := current.directory
		if directory == "" {
			directory = repo.Root()
		}
		if err := repo.Run(directory, current.command[0], current.command[1:]...); err != nil {
			repo.Fail(fmt.Errorf("%s failed", current.name))
		}
	}
	if len(skipped) > 0 {
		fmt.Printf("Skipped (tool or target not installed): %s\n", strings.Join(skipped, ", "))
	}
	fmt.Println("Preflight passed")
}

func available(current step) bool {
	if _, err := exec.LookPath(current.command[0]); err != nil {
		return false
	}
	for index, argument := range current.command {
		if argument == "--target" && index+1 < len(current.command) {
			return targetInstalled(current.command[index+1])
		}
	}
	if current.command[0] == "cargo" && len(current.command) > 1 && current.command[1] == "deny" {
		return exec.Command("cargo", "deny", "--version").Run() == nil
	}
	return true
}

func targetInstalled(target string) bool {
	output, err := exec.Command("rustup", "target", "list", "--installed").Output()
	if err != nil {
		return false
	}
	return strings.Contains(string(output), target)
}

func init() {
	if len(os.Args) > 1 {
		repo.Fail(fmt.Errorf("preflight takes no arguments"))
	}
}
