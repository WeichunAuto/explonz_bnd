# Axum Web Project Template Makefile
# This Makefile provides various development commands for the Axum web project

# Phony targets (targets that are not files)
.PHONY: help install install-tools setup dev test check clean reset db-migrate db-reset db-setup pre-commit

# Variable definitions
CARGO := cargo
PROJECT_NAME := $(shell basename $(CURDIR))

# Color definitions for terminal output
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
BLUE := \033[0;34m
CYAN := \033[0;36m
MAGENTA := \033[0;35m
NC := \033[0m # No Color (reset)

# Progress symbols
CHECKMARK := ✓
ARROW := →
INFO := ℹ️

# Help information - main commands
help:
	@echo "$(CYAN)=== Axum Web Project Template ===$(NC)"
	@echo ""
	@echo "$(YELLOW)Available Commands:$(NC)"
	@echo "  $(GREEN)make install$(NC)          - Install all development tools (with detailed progress)"
	@echo "  $(GREEN)make install-tools$(NC)    - Install only Rust tools"
	@echo "  $(GREEN)make setup$(NC)            - Project initialization setup"
	@echo "  $(GREEN)make dev$(NC)              - Start development server (auto-reload)"
	@echo "  $(GREEN)make test$(NC)             - Run tests"
	@echo "  $(GREEN)make check$(NC)            - Run code quality checks"
	@echo ""
	@echo "$(YELLOW)Database Commands:$(NC)"
	@echo "  $(GREEN)make db-setup$(NC)         - Setup database (migrations + seed data)"
	@echo "  $(GREEN)make db-migrate$(NC)       - Run database migrations"
	@echo "  $(GREEN)make db-reset$(NC)         - Reset database"
	@echo "  $(GREEN)make db-status$(NC)        - Check migration status"
	@echo ""
	@echo "$(YELLOW)Code Quality:$(NC)"
	@echo "  $(GREEN)make pre-commit$(NC)       - Run pre-commit checks"
	@echo "  $(GREEN)make spell-check$(NC)      - Run spell checking"
	@echo "  $(GREEN)make clean$(NC)            - Clean project"
	@echo ""
	@echo "$(YELLOW)Tool Checks:$(NC)"
	@echo "  $(GREEN)make check-tools$(NC)      - Check if all tools are installed"
	@echo "  $(GREEN)make version$(NC)          - Display tool version information"

# Check Rust installation
check-rust:
	@echo "$(BLUE)🔍 Checking Rust installation...$(NC)"
	@which rustup > /dev/null 2>&1 || (echo "$(RED)❌ Error: Rust not installed, please install Rust first: https://rustup.rs/$(NC)" && exit 1)
	@echo "$(GREEN)$(CHECKMARK) Rust is installed$(NC)"

# Function to check if a tool is installed
define check-tool-installed
	@if command -v $(1) > /dev/null 2>&1; then \
		echo "$(GREEN)[$(2)/$(3)] $(CHECKMARK) $(4) is installed$(NC)"; \
		true; \
	else \
		false; \
	fi
endef

# Function to install tools
define install-tool
	@if command -v $(1) > /dev/null 2>&1; then \
		echo "$(GREEN)[$(5)/$(6)] $(CHECKMARK) $(4) is installed$(NC)"; \
	else \
		echo "$(YELLOW)[$(5)/$(6)] $(ARROW) Installing $(4)...$(NC)"; \
		if $(CARGO) install $(2) $(3); then \
			echo "$(GREEN)[$(5)/$(6)] $(CHECKMARK) $(4) installed successfully$(NC)"; \
		else \
			echo "$(RED)[$(5)/$(6)] ❌ $(4) installation failed (possible network issue or version conflict)$(NC)"; \
		fi; \
	fi
endef

# Specialized function to install pgcli
define install-pgcli
	@if command -v pgcli > /dev/null 2>&1; then \
		echo "$(GREEN)[8/8] $(CHECKMARK) pgcli is installed$(NC)"; \
	elif command -v brew > /dev/null 2>&1; then \
		echo "$(YELLOW)[8/8] $(ARROW) Installing pgcli via Homebrew...$(NC)"; \
		if brew install pgcli; then \
			echo "$(GREEN)[8/8] $(CHECKMARK) pgcli installed successfully (via Homebrew)$(NC)"; \
		else \
			echo "$(RED)[8/8] ❌ Homebrew installation failed, trying pip...$(NC)"; \
			if pip install pgcli; then \
				echo "$(GREEN)[8/8] $(CHECKMARK) pgcli installed successfully (via pip)$(NC)"; \
			else \
				echo "$(RED)[8/8] ❌ pgcli installation failed$(NC)"; \
			fi; \
		fi; \
	elif command -v pip > /dev/null 2>&1; then \
		echo "$(YELLOW)[8/8] $(ARROW) Installing pgcli via pip...$(NC)"; \
		if pip install pgcli; then \
			echo "$(GREEN)[8/8] $(CHECKMARK) pgcli installed successfully$(NC)"; \
		else \
			echo "$(RED)[8/8] ❌ pgcli installation failed$(NC)"; \
		fi; \
	else \
		echo "$(YELLOW)[8/8] ⚠️  Skipping pgcli installation (brew or pip not found)$(NC)"; \
		echo "$(CYAN)💡 Tip: Please install pgcli manually:$(NC)"; \
		echo "  brew install pgcli or pip install pgcli"; \
	fi
endef

# Specialized function to install sqlx with PostgreSQL features
define install-sqlx
	@if command -v sqlx > /dev/null 2>&1; then \
		echo "$(GREEN)[6/7] $(CHECKMARK) sqlx-cli is installed$(NC)"; \
	else \
		echo "$(YELLOW)[6/7] $(ARROW) Installing sqlx-cli...$(NC)"; \
		if $(CARGO) install sqlx-cli --no-default-features --features native-tls,postgres; then \
			echo "$(GREEN)[6/7] $(CHECKMARK) sqlx-cli installed successfully$(NC)"; \
		else \
			echo "$(RED)[6/7] ❌ sqlx-cli installation failed$(NC)"; \
		fi; \
	fi
endef

# Check all development tools
check-tools:
	@echo "$(BLUE)🔍 Checking development tool installation status...$(NC)"
	@$(MAKE) check-rust
	@echo "$(YELLOW)$(INFO) Rust Tools Check:$(NC)"
	@$(call check-tool-installed,cargo-generate,-,-,cargo-generate)
	@$(call check-tool-installed,typos,-,-,typos)
	@$(call check-tool-installed,git-cliff,-,-,git-cliff)
	@$(call check-tool-installed,cargo-watch,-,-,cargo-watch)
	@$(call check-tool-installed,sea-orm-cli,-,-,sea-orm-cli)
	@$(call check-tool-installed,sqlx,-,-,sqlx-cli)
	@$(call check-tool-installed,cargo-audit,-,-,cargo-audit)
	@echo ""
	@echo "$(YELLOW)$(INFO) Other Tools Check:$(NC)"
	@if command -v pre-commit > /dev/null 2>&1; then \
		echo "$(GREEN)$(CHECKMARK) pre-commit is installed$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  pre-commit is not installed$(NC)"; \
	fi
	@if command -v pgcli > /dev/null 2>&1; then \
		echo "$(GREEN)$(CHECKMARK) pgcli is installed$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  pgcli is not installed$(NC)"; \
	fi

# Install all development tools
install: install-tools install-pre-commit install-pgcli
	@echo ""
	@echo "$(GREEN)🎉 All development tools installed successfully!$(NC)"
	@echo ""
	@echo "$(YELLOW)📝 Next steps:$(NC)"
	@echo "  Run 'make setup' for project initialization"

# Install Rust tools
install-tools: check-rust
	@echo "$(MAGENTA)🛠️  Starting Rust development tools installation...$(NC)"
	@echo "$(BLUE)========================================$(NC)"

	$(call install-tool,cargo-generate,cargo-generate,,cargo-generate,1,7)
	$(call install-tool,typos,typos-cli,,typos,2,7)
	$(call install-tool,git-cliff,git-cliff,,git-cliff,3,7)
	$(call install-tool,cargo-watch,cargo-watch,,cargo-watch,4,7)
	$(call install-tool,sea-orm-cli,sea-orm-cli,,sea-orm-cli,5,7)
	@$(call install-sqlx)  # Use specialized installation function
	$(call install-tool,cargo-audit,cargo-audit,,cargo-audit,7,7)

	@echo "$(BLUE)========================================$(NC)"
	@echo "$(GREEN)✅ Rust tools installation completed$(NC)"

# Install pgcli
install-pgcli:
	@echo ""
	@echo "$(MAGENTA)🐘 Installing PostgreSQL client tools...$(NC)"
	@echo "$(BLUE)========================================$(NC)"
	@$(call install-pgcli)
	@echo "$(BLUE)========================================$(NC)"
	@echo "$(GREEN)✅ PostgreSQL tools installation completed$(NC)"

# Install pre-commit
install-pre-commit:
	@echo ""
	@echo "$(MAGENTA)🔧 Setting up pre-commit...$(NC)"
	@if command -v pre-commit > /dev/null 2>&1; then \
		echo "$(GREEN)$(CHECKMARK) pre-commit is installed$(NC)"; \
		if [ -f .pre-commit-config.yaml ]; then \
			echo "$(YELLOW)$(ARROW) Installing pre-commit hook...$(NC)"; \
			pre-commit install; \
			echo "$(GREEN)$(CHECKMARK) pre-commit hook installed$(NC)"; \
		else \
			echo "$(YELLOW)⚠️  Warning: .pre-commit-config.yaml not found$(NC)"; \
		fi \
	else \
		echo "$(YELLOW)⚠️  Warning: pre-commit not installed, skipping$(NC)"; \
		echo "$(CYAN)💡 Tip: To install pre-commit, run:$(NC)"; \
		echo "  pip install pre-commit or pipx install pre-commit"; \
	fi

# Project initialization setup
setup: db-setup build
	@echo ""
	@echo "$(GREEN)✅ Project setup completed!$(NC)"
	@echo ""
	@echo "$(CYAN)🚀 You can run the following commands:$(NC)"
	@echo "  make dev    - Start development server"
	@echo "  make test   - Run tests"
	@echo "  make check  - Code quality checks"

# Build project
build:
	@echo "$(BLUE)🔨 Building project...$(NC)"
	@$(CARGO) build
	@echo "$(GREEN)$(CHECKMARK) Project build completed$(NC)"

# Database setup
db-setup: db-migrate
	@echo "$(GREEN)$(CHECKMARK) Database setup completed$(NC)"

# Database migration (supports sqlx)
db-migrate:
	@echo "$(BLUE)🗃️  Running database migrations...$(NC)"
	@if [ -f .env ]; then \
		if command -v sqlx > /dev/null 2>&1; then \
			echo "$(YELLOW)$(ARROW) Checking database axum_template...$(NC)"; \
			if psql -lqt | cut -d \| -f 1 | grep -qw axum_template; then \
				echo "$(YELLOW)$(ARROW) Database exists, dropping...$(NC)"; \
				dropdb axum_template; \
			fi; \
			echo "$(YELLOW)$(ARROW) Creating database axum_template...$(NC)"; \
			createdb axum_template; \
			echo "$(GREEN)$(CHECKMARK) Database creation completed$(NC)"; \
			\
			echo "$(YELLOW)$(ARROW) Creating migration file...$(NC)"; \
			sqlx migrate add init; \
			\
			echo "$(YELLOW)$(ARROW) Copying SQL content to migration file...$(NC)"; \
			if [ -f sql/init.sql ]; then \
				latest_migration=$$(find migrations -name "*_init.sql" -type f | sort -r | head -n 1); \
				if [ -n "$$latest_migration" ]; then \
					cat sql/init.sql >> "$$latest_migration"; \
					echo "$(GREEN)$(CHECKMARK) SQL content copied successfully$(NC)"; \
				else \
					echo "$(RED)❌ Migration file not found$(NC)"; \
				fi; \
			else \
				echo "$(RED)❌ sql/init.sql file not found$(NC)"; \
			fi; \
			\
			echo "$(YELLOW)$(ARROW) Executing database migrations...$(NC)"; \
			sqlx migrate run; \
			echo "$(GREEN)$(CHECKMARK) sqlx migration completed$(NC)"; \
			\
			if command -v sea-orm-cli > /dev/null 2>&1; then \
			  	echo "$(YELLOW)$(ARROW) Creating entity directory...$(NC)"; \
              	mkdir -p ./src/entity; \
				echo "$(YELLOW)$(ARROW) Generating SeaORM entities...$(NC)"; \
				sea-orm-cli generate entity -s public --with-serde both --model-extra-attributes 'serde(rename_all="camelCase")' --date-time-crate chrono -o ./src/entity; \
				echo "$(GREEN)$(CHECKMARK) SeaORM entity generation completed$(NC)"; \
			else \
				echo "$(YELLOW)⚠️  sea-orm-cli not installed, skipping entity generation$(NC)"; \
			fi; \
		else \
			echo "$(RED)❌ Error: sqlx not installed$(NC)"; \
		fi \
	else \
		echo "$(YELLOW)⚠️  .env file not found, skipping database migration$(NC)"; \
		echo "$(CYAN)💡 Tip: Please copy .env.example to .env and configure database connection$(NC)"; \
	fi

# Start development server (auto-reload)
dev:
	@echo "$(BLUE)🚀 Starting development server (auto-reload)...$(NC)"
	@echo "$(YELLOW)📡 Watching file changes and auto-restarting$(NC)"
	@echo "$(YELLOW)⏹️  Use Ctrl+C to stop server$(NC)"
	@cargo watch -x 'run'

# Run tests
test:
	@echo "$(BLUE)🧪 Running test suite...$(NC)"
	@$(CARGO) test -- --nocapture

# Code quality checks
check: spell-check pre-commit cargo-check
	@echo "$(GREEN)✅ All code checks completed!$(NC)"

# Spell checking
spell-check:
	@echo "$(BLUE)📝 Running spell check...$(NC)"
	@if command -v typos > /dev/null 2>&1; then \
		typos; \
		echo "$(GREEN)$(CHECKMARK) Spell check completed$(NC)"; \
	else \
		echo "$(YELLOW)⚠️  Warning: typos not installed, skipping spell check$(NC)"; \
	fi

# pre-commit checks
pre-commit:
	@echo "$(BLUE)🔍 Running pre-commit checks...$(NC)"
	@if command -v pre-commit > /dev/null 2>&1; then \
		pre-commit run --all-files; \
	else \
		echo "$(YELLOW)⚠️  Warning: pre-commit not installed, skipping checks$(NC)"; \
	fi

# Cargo checks
cargo-check:
	@echo "$(BLUE)🔧 Running cargo checks...$(NC)"
	@$(CARGO) check
	@$(CARGO) clippy -- -D warnings
	@echo "$(GREEN)$(CHECKMARK) Cargo checks completed$(NC)"

# Clean project
clean:
	@echo "$(BLUE)🧹 Cleaning project...$(NC)"
	@$(CARGO) clean
	@echo "$(GREEN)$(CHECKMARK) Project cleaned$(NC)"

# Display version information
version:
	@echo "$(CYAN)📋 Tool Version Information $(NC)"
	@echo "$(BLUE)================$(NC)"
	@$(CARGO) --version || echo "$(RED)❌ cargo: not installed$(NC)"
	@rustup --version || echo "$(RED)❌ rustup: not installed$(NC)"
	@$(CARGO) generate --version 2>/dev/null || echo "$(YELLOW)⚠️  cargo-generate: not installed$(NC)"
	@pre-commit --version 2>/dev/null || echo "$(YELLOW)⚠️  pre-commit: not installed$(NC)"
	@typos --version 2>/dev/null || echo "$(YELLOW)⚠️  typos: not installed$(NC)"
	@git-cliff --version 2>/dev/null || echo "$(YELLOW)⚠️  git-cliff: not installed$(NC)"
	@cargo-watch --version 2>/dev/null || echo "$(YELLOW)⚠️  cargo-watch: not installed$(NC)"
	@sea-orm-cli --version 2>/dev/null || echo "$(YELLOW)⚠️  sea-orm-cli: not installed$(NC)"
	@sqlx --version 2>/dev/null || echo "$(YELLOW)⚠️  sqlx-cli: not installed$(NC)"
	@if command -v pgcli > /dev/null 2>&1; then \
		pgcli --version | awk '{print "pgcli", $$NF}'; \
	else \
		echo "$(YELLOW)⚠️  pgcli: not installed$(NC)"; \
	fi
