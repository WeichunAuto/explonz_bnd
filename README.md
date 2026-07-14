# Axum Web Project Template

This repository aims to provide a **production-ready project template** for building robust web applications with Rust and the Axum framework. It encapsulates industry best practices and common development patterns to accelerate your project setup while ensuring maintainability and scalability.

## 🎯 What can you get?

- **🚀 Best Practices Structure**: Well-organized project architecture following Rust and Axum conventions, providing a solid foundation for growing applications.

- **📦 Unified API Response Format**: Pre-defined response structures that standardize API output, ensuring consistency across all endpoints.

- **🔍 Integrated Tracing and Logging**: Built-in tracing middleware with automatic Request ID generation for every incoming request, enabling efficient log correlation and rapid issue diagnosis.

- **🗄️ SeaORM Integration**: Seamlessly integrated with SeaORM for type-safe database operations, basic examples covering GET, POST, PATCH, and DELETE requests with path, request validation, query parameters, and payload body.

- **⚙️ Dev and Prod separated Configuration**: Support for separate development and production environment configurations.

---

## How to use ?
This is an **SQL-first Axum WEB project template** that provides a solid foundation for building database-driven web applications. The template includes comprehensive CRUD operations for a user management example with associated workspaces, demonstrating real-world data relationships and API patterns.

### 1. Generate a new project from this template:
```bash
cargo generate https://github.com/WeichunAuto/axum-template --name your-project-name
```
### 2. Design Your Database Schema
Define your database structure using SQL-first approach in the `sql/init.sql` file:

### 3. Verify Development Environment
Check if all required development tools are properly installed:

```bash
make version
```
This command displays the version information for all essential tools, any missing tools will be clearly marked, allowing you to install them before proceeding.

### 4. Install all required development tools.

```bash
make install
```

### 5. Set up and build the project
**First, configure your database credentials:**
1. Edit `config/dev.yaml or config/prod.yaml` if you are in the production environment and update the database username:

```yaml
database:
  user: "your_user_name"
  password: "your_password"
```
2. Update .env file with your database connection:
```env
DATABASE_URL=postgres://your_username:your_password@localhost:5432/axum_template
```

**Then run the setup command:**

```bash
make setup
```
This command automates the complete project initialization process:

- **Database Migration:** Automatically generates and executes migration scripts based on your sql/init.sql schema definition

- **Database Creation:** Creates a PostgreSQL database named axum_template

- **Sample Data:** If you use the default *./sql/init.sql* without modifications, the database will be populated with: users table with sample user records, workspaces table with sample workspace data. The realistic seed data for immediate development and testing

- **Entity Generation:** Automatically generates SeaORM entities from your database schema

- **Project Build:** Compiles the entire project with all dependencies

### 6. Start the project
```bash
make dev
```
This command starts the server with automatic hot-reload; any code changes will trigger an immediate restart, allowing you to see updates in real-time without manual intervention.

### Alternative startup methods:

```bash
# Standard production build
cargo run

# Manual watch mode (equivalent to make dev)
cargo watch -x run
```
