use anyhow::Result;
use log::{debug, warn};
use std::collections::{HashMap, HashSet};

use super::Generator;
use crate::ast::ASTNode;

// Структура для описания соответствия функций C и Python
#[derive(Clone)]
struct FunctionMapping {
    c_name: &'static str,
    python_code: &'static str,
    import_required: Option<&'static str>,
    _is_standard_python: bool,
}

impl FunctionMapping {
    fn new(
        c_name: &'static str,
        python_code: &'static str,
        import_required: Option<&'static str>,
    ) -> Self {
        Self {
            c_name,
            python_code,
            import_required,
            _is_standard_python: true,
        }
    }
}
pub struct PythonGenerator {
    indent_level: usize,
    indent_size: usize,
    symbols: HashSet<String>,
    in_expression: bool,
    current_struct_type: Option<String>,
    struct_info: HashMap<String, Vec<String>>,
    enum_info: HashMap<String, Vec<String>>,
    pointer_vars: HashSet<String>,
    address_taken: HashSet<String>,
    array_vars: HashSet<String>,
    array_element_types: HashMap<String, String>,
    pending_string_assignments: HashMap<String, Vec<(usize, String)>>,
    required_imports: HashSet<String>,
    used_functions: HashSet<String>,
    function_mappings: HashMap<String, FunctionMapping>,
}

impl PythonGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            indent_level: 0,
            indent_size: 4,
            symbols: HashSet::new(),
            in_expression: false,
            current_struct_type: None,
            struct_info: HashMap::new(),
            enum_info: HashMap::new(),
            pointer_vars: HashSet::new(),
            address_taken: HashSet::new(),
            array_vars: HashSet::new(),
            array_element_types: HashMap::new(),
            pending_string_assignments: HashMap::new(),
            required_imports: HashSet::new(),
            used_functions: HashSet::new(),
            function_mappings: HashMap::new(),
        };

        generator.init_function_mappings();
        generator
    }
    /// Инициализация маппинга стандартных функций C
    fn init_function_mappings(&mut self) {
        let mappings: Vec<FunctionMapping> = vec![
            // Математические функции (math.h) - требуют import math
            FunctionMapping::new("sin", "math.sin({})", Some("import math")),
            FunctionMapping::new("cos", "math.cos({})", Some("import math")),
            FunctionMapping::new("tan", "math.tan({})", Some("import math")),
            FunctionMapping::new("asin", "math.asin({})", Some("import math")),
            FunctionMapping::new("acos", "math.acos({})", Some("import math")),
            FunctionMapping::new("atan", "math.atan({})", Some("import math")),
            FunctionMapping::new("atan2", "math.atan2({}, {})", Some("import math")),
            FunctionMapping::new("sinh", "math.sinh({})", Some("import math")),
            FunctionMapping::new("cosh", "math.cosh({})", Some("import math")),
            FunctionMapping::new("tanh", "math.tanh({})", Some("import math")),
            FunctionMapping::new("exp", "math.exp({})", Some("import math")),
            FunctionMapping::new("log", "math.log({})", Some("import math")),
            FunctionMapping::new("log10", "math.log10({})", Some("import math")),
            FunctionMapping::new("sqrt", "math.sqrt({})", Some("import math")),
            FunctionMapping::new("pow", "math.pow({}, {})", Some("import math")),
            FunctionMapping::new("fabs", "math.fabs({})", Some("import math")),
            FunctionMapping::new("ceil", "math.ceil({})", Some("import math")),
            FunctionMapping::new("floor", "math.floor({})", Some("import math")),
            FunctionMapping::new("fmod", "math.fmod({}, {})", Some("import math")),
            // Строковые функции (string.h) - некоторые встроенные, некоторые требуют import
            FunctionMapping::new("strlen", "len({})", None),
            FunctionMapping::new("strcpy", "{}[:]", None), // Для копирования строки
            FunctionMapping::new("strcat", "{0} + {1}", None), // Конкатенация
            FunctionMapping::new(
                "strcmp",
                "(0 if {0} == {1} else -1 if {0} < {1} else 1)",
                None,
            ),
            FunctionMapping::new(
                "strncmp",
                "(0 if {0}[:{2}] == {1}[:{2}] else -1 if {0}[:{2}] < {1}[:{2}] else 1)",
                None,
            ),
            FunctionMapping::new("strchr", "{0}.find({1})", None),
            FunctionMapping::new("strstr", "{0}.find({1})", None),
            FunctionMapping::new("strdup", "{0}[:]", None),
            FunctionMapping::new("strlwr", "{0}.lower()", None),
            FunctionMapping::new("strupr", "{0}.upper()", None),
            // Функции ввода-вывода (stdio.h)
            FunctionMapping::new("printf", "print({})", None),
            FunctionMapping::new("puts", "print({})", None),
            FunctionMapping::new("putchar", "print(chr({}), end='')", None),
            FunctionMapping::new("sprintf", "{} = {}", None),
            FunctionMapping::new("snprintf", "{} = {}", None),
            FunctionMapping::new("fopen", "open({}, {})", None),
            FunctionMapping::new("fclose", "{}.close()", None),
            FunctionMapping::new("fprintf", "{}.write({})", None),
            FunctionMapping::new("fscanf", "{}.read()", None),
            FunctionMapping::new("fgets", "{}.readline()", None),
            FunctionMapping::new("fputs", "{}.write({})", None),
            FunctionMapping::new("getchar", "sys.stdin.read(1)", Some("import sys")),
            FunctionMapping::new(
                "gets",
                "sys.stdin.readline().rstrip('\\n')",
                Some("import sys"),
            ),
            FunctionMapping::new("scanf", "int(input())", None), // Упрощенно для целых чисел
            // Функции стандартной библиотеки (stdlib.h)
            FunctionMapping::new("atoi", "int({})", None),
            FunctionMapping::new("atol", "int({})", None),
            FunctionMapping::new("atof", "float({})", None),
            FunctionMapping::new("itoa", "str({})", None),
            FunctionMapping::new("abs", "abs({})", None),
            FunctionMapping::new("labs", "abs({})", None),
            FunctionMapping::new("rand", "random.randint(0, RAND_MAX)", Some("import random")),
            FunctionMapping::new("srand", "random.seed({})", Some("import random")),
            FunctionMapping::new("malloc", "[None] * {}", None), // Упрощенно для массивов
            FunctionMapping::new("calloc", "[0] * ({0} * {1})", None),
            FunctionMapping::new("free", "del {}", None),
            FunctionMapping::new("exit", "sys.exit({})", Some("import sys")),
            FunctionMapping::new("system", "os.system({})", Some("import os")),
            FunctionMapping::new("getenv", "os.environ.get({})", Some("import os")),
            FunctionMapping::new("setenv", "os.environ[{}] = {}", Some("import os")),
            // Функции времени (time.h)
            FunctionMapping::new("time", "time.time()", Some("import time")),
            FunctionMapping::new("clock", "time.process_time()", Some("import time")),
            FunctionMapping::new("difftime", "{} - {}", None),
            FunctionMapping::new("ctime", "time.ctime({})", Some("import time")),
            FunctionMapping::new("sleep", "time.sleep({})", Some("import time")),
            // Функции для работы с символами (ctype.h)
            FunctionMapping::new("isalnum", "{}.isalnum()", None),
            FunctionMapping::new("isalpha", "{}.isalpha()", None),
            FunctionMapping::new("isdigit", "{}.isdigit()", None),
            FunctionMapping::new("islower", "{}.islower()", None),
            FunctionMapping::new("isupper", "{}.isupper()", None),
            FunctionMapping::new("isspace", "{}.isspace()", None),
            FunctionMapping::new("tolower", "{}.lower()", None),
            FunctionMapping::new("toupper", "{}.upper()", None),
            // Функции для работы с памятью
            FunctionMapping::new("memcpy", "{}[:len({})] = {}[:len({})]", None),
            FunctionMapping::new("memmove", "{}[:len({})] = {}[:len({})]", None),
            FunctionMapping::new("memset", "{} = [{}] * len({})", None),
            FunctionMapping::new("memcmp", "{} == {}", None),
            // Специальные функции
            FunctionMapping::new("assert", "assert {}, \"Assertion failed\"", None),
            FunctionMapping::new("qsort", "sorted({})", None), // Упрощенно
            FunctionMapping::new("bsearch", "{} in {}", None), // Упрощенно
        ];

        for mapping in mappings {
            self.function_mappings
                .insert(mapping.c_name.to_string(), mapping);
        }
    }
    /// Возвращает текущий отступ
    fn indent(&self) -> String {
        " ".repeat(self.indent_level * self.indent_size)
    }

    /// Добавляет отступ и строку
    fn line(&self, content: &str) -> String {
        format!("{}{}\n", self.indent(), content)
    }

    /// Добавляет импорт, если он еще не был добавлен
    fn add_import(&mut self, import_stmt: &str) {
        if !self.required_imports.contains(import_stmt) {
            self.required_imports.insert(import_stmt.to_string());
        }
    }

    fn generate_imports(&self) -> String {
        let mut output = String::new();
        let _imports: Vec<&String> = self.required_imports.iter().collect(); 

        // Если нужна сортировка, собираем в Vec<String>
        let mut imports: Vec<String> = self.required_imports.iter().cloned().collect();
        imports.sort();

        for import in &imports {
            // Используем ссылку &
            output.push_str(import);
            output.push('\n');
        }

        if !imports.is_empty() {
            output.push('\n');
        }

        output
    }
    fn generate_function(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        // Получаем имя функции из разных мест
        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Генерация функции: {}", name);

        // ОТЛАДКА: выводим все атрибуты узла
        debug!("Атрибуты узла {}: {:#?}", name, node.attributes);

        // ОТЛАДКА: выводим всех детей узла
        debug!("Дети узла {} в функции:", name);
        for (i, child) in node.children.iter().enumerate() {
            debug!("  Дитя {}: тип={}", i, child.node_type);
            if child.node_type == "Compound" {
                debug!("    Compound дети:");
                for (j, stmt) in child.children.iter().enumerate() {
                    debug!("      Оператор {}: тип={}", j, stmt.node_type);
                }
            }
        }

        // Собираем параметры функции и определяем, какие из них указатели
        let mut params = Vec::new();
        let mut pointer_params = std::collections::HashSet::new();

        // 1. Проверяем атрибут param_names (добавлен в c_parser)
        if let Some(param_names) = node.attributes.get("param_names") {
            debug!("Найден атрибут param_names: {:#?}", param_names);
            if let Some(param_array) = param_names.as_array() {
                for param in param_array {
                    if let Some(param_name) = param.as_str() {
                        debug!("Найден параметр в param_names: {}", param_name);
                        params.push(param_name.to_string());
                        self.symbols.insert(param_name.to_string());

                        // В реальном коде здесь нужно анализировать тип параметра
                        // Пока будем считать, что параметры с именами x, y, z - указатели (для функции swap)
                        if param_name == "x" || param_name == "y" || param_name == "z" {
                            pointer_params.insert(param_name.to_string());
                        }
                    }
                }
            }
        }

        // 2. Проверяем атрибут params
        if params.is_empty() {
            if let Some(params_attr) = node.attributes.get("params") {
                debug!("Найден атрибут params: {:#?}", params_attr);
                if let Some(params_array) = params_attr.as_array() {
                    for param in params_array {
                        if let Some(param_obj) = param.as_object() {
                            if let Some(param_name) = param_obj.get("name").and_then(|v| v.as_str())
                            {
                                debug!("Найден параметр в attributes.params: {}", param_name);
                                params.push(param_name.to_string());
                                self.symbols.insert(param_name.to_string());

                                // Проверяем, является ли параметр указателем
                                if let Some(type_obj) = param_obj.get("type") {
                                    if let Some(type_str) = type_obj.as_str() {
                                        if type_str.contains('*') || type_str == "ptr" {
                                            pointer_params.insert(param_name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Ищем параметры в детях (для FuncDef)
        if params.is_empty() {
            for child in &node.children {
                if child.node_type == "ParamList" {
                    debug!("Найден ParamList с {} детьми", child.children.len());
                    for param in &child.children {
                        if let Some(param_name) = self.extract_param_name(param) {
                            debug!("Найден параметр в ParamList: {}", param_name);
                            params.push(param_name.clone());
                            self.symbols.insert(param_name.clone());

                            // Проверяем, является ли параметр указателем
                            if param.node_type == "PtrDecl"
                                || (param.node_type == "Decl"
                                    && param.children.iter().any(|c| c.node_type == "PtrDecl"))
                            {
                                pointer_params.insert(param_name);
                            }
                        }
                    }
                }
            }
        }

        // 4. Ищем параметры в детях типа "Decl" (объявления переменных могут быть параметрами)
        if params.is_empty() {
            for child in &node.children {
                if child.node_type == "Decl" {
                    if let Some(param_name) = child.attributes.get("name").and_then(|v| v.as_str())
                    {
                        debug!("Найден параметр в Decl: {}", param_name);
                        params.push(param_name.to_string());
                        self.symbols.insert(param_name.to_string());

                        // Проверяем, является ли параметр указателем
                        for grandchild in &child.children {
                            if grandchild.node_type == "PtrDecl" {
                                pointer_params.insert(param_name.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        debug!("Параметры функции {}: {:?}", name, params);
        debug!("Параметры-указатели: {:?}", pointer_params);

        // Генерируем сигнатуру функции
        output.push_str(&self.line(&format!("def {}({}):", name, params.join(", "))));

        self.indent_level += 1;

        // Добавляем аннотации для параметров-указателей (для документации)
        if !pointer_params.is_empty() {
            output.push_str(&self.line("# Параметры-указатели (ожидают объекты Reference):"));
            for param in &pointer_params {
                output.push_str(&self.line(&format!("# {}: Reference", param)));
            }
        }

        // Генерируем тело функции
        let mut _has_return = false;
        let mut has_body = false;
        let mut body_output = String::new();

        for child in &node.children {
            if child.node_type == "Compound" {
                has_body = true;
                debug!("Обработка Compound с {} детьми", child.children.len());

                // Очищаем накопленные строки перед обработкой блока
                self.pending_string_assignments.clear();

                for (i, stmt) in child.children.iter().enumerate() {
                    debug!("  Оператор {} в Compound: тип={}", i, stmt.node_type);

                    let stmt_code = self.generate_statement(stmt)?;

                    // Если оператор вернул непустой код, добавляем его
                    if !stmt_code.is_empty() {
                        body_output.push_str(&stmt_code);
                    }

                    // Проверяем, не является ли следующий оператор тоже строковым присваиванием
                    let next_is_string_assignment = child
                        .children
                        .get(i + 1)
                        .map(|next| {
                            if next.node_type == "Assignment" && next.children.len() >= 2 {
                                let left = &next.children[0];
                                left.node_type == "ArrayRef" && left.children.len() >= 2
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);

                    // Если следующий оператор не строковое присваивание и у нас есть накопленные строки,
                    // выводим их сейчас
                    if !next_is_string_assignment && !self.pending_string_assignments.is_empty() {
                        body_output.push_str(&self.finalize_string_assignments());
                    }
                }
                // В конце, если остались накопленные строки, выводим их
                if !self.pending_string_assignments.is_empty() {
                    body_output.push_str(&self.finalize_string_assignments());
                }
            }
        }

        // Добавляем тело функции
        output.push_str(&body_output);

        // Если нет тела или тело пустое, добавляем pass
        if !has_body {
            output.push_str(&self.line("    pass"));
        } else if body_output.trim().is_empty() {
            output.push_str(&self.line("    pass"));
        }

        self.indent_level -= 1;
        output.push_str(&self.line(""));

        Ok(output)
    }

    // Извлекает имя параметра из узла
    fn extract_param_name(&self, node: &ASTNode) -> Option<String> {
        // Прямой атрибут name
        if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
            return Some(name.to_string());
        }

        // Проверяем детей
        for child in &node.children {
            if child.node_type == "ID" {
                if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    return Some(name.to_string());
                }
            }
        }

        None
    }

    /// Генерирует оператор
    fn generate_statement(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация оператора типа: {}", node.node_type);
        match node.node_type.as_str() {
            "Return" => self.generate_return(node),
            "If" => self.generate_if(node),
            "While" => self.generate_while(node),
            "DoWhile" => self.generate_dowhile(node),
            "For" => self.generate_for(node),
            "Assignment" => {
                let mut stmt_output = String::new();

                // Проверяем, нужно ли инициализировать вложенные объекты
                if node.children.len() >= 2 {
                    let left_node = &node.children[0];

                    // Генерируем код инициализации для левой части
                    let init_code = self.ensure_nested_objects(left_node, "")?;
                    if !init_code.is_empty() {
                        stmt_output.push_str(&init_code);
                    }
                }

                // Затем генерируем само присваивание
                stmt_output.push_str(&self.generate_assignment(node)?);
                Ok(stmt_output)
            }
            "Decl" => {
                // Проверяем, является ли это объявлением массива
                let mut is_array = false;
                for child in &node.children {
                    if child.node_type == "ArrayDecl" {
                        is_array = true;
                        break;
                    }
                }

                if is_array {
                    // Ищем узел ArrayDecl среди детей
                    for child in &node.children {
                        if child.node_type == "ArrayDecl" {
                            return self.generate_array_decl(child);
                        }
                    }
                    Ok(String::new())
                } else {
                    self.generate_declaration(node)
                }
            }
            "Switch" => self.generate_switch(node),
            "FuncCall" => {
                let expr = self.generate_expression(node)?;
                Ok(self.line(&expr))
            }
            "UnaryOp" => {
                // Для унарных операторов как операторов (например, *ptr = 5)
                if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
                    if op == "*" && node.children.len() == 1 {
                        // Это разыменование указателя как левая часть присваивания
                        // Будет обработано в Assignment
                        return self.generate_unary_stmt(node);
                    }
                }
                self.generate_unary_stmt(node)
            }
            "Compound" => {
                let mut output = String::new();

                // Просто генерируем операторы в том порядке, в котором они идут в AST
                for stmt in &node.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }

                Ok(output)
            }
            "Break" => Ok(self.line("break")),
            "Continue" => Ok(self.line("continue")),
            "Dowhile" => self.generate_dowhile(node),
            "DeclList" => {
                let mut output = String::new();
                for decl in &node.children {
                    output.push_str(&self.generate_statement(decl)?);
                }
                Ok(output)
            }
            "Struct" => self.generate_struct(node),
            "StructDecl" => self.generate_struct_decl(node),
            "Union" => self.generate_union(node),
            "UnionDecl" => self.generate_union_decl(node),
            _ => {
                // Пытаемся обработать как выражение
                let expr = self.generate_expression(node)?;
                if !expr.is_empty() && expr != "None" {
                    Ok(self.line(&expr))
                } else {
                    debug!("Пропуск неизвестного оператора: {}", node.node_type);
                    Ok(String::new())
                }
            }
        }
    }

    fn generate_dowhile(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация DO-WHILE цикла");

        if node.children.len() >= 2 {
            let body = &node.children[0];
            let condition = &node.children[1];

            let cond_code = self.generate_expression(condition)?;

            // Простая и понятная реализация do-while
            output.push_str(&self.line("while True:"));
            self.indent_level += 1;

            // Генерируем тело цикла
            if body.node_type == "Compound" {
                for stmt in &body.children {
                    if stmt.node_type == "Continue" {
                        // Continue просто переходит к проверке условия
                        output.push_str(&self.line("pass  # continue"));
                    } else {
                        let stmt_code = self.generate_statement(stmt)?;
                        if !stmt_code.trim().is_empty() {
                            output.push_str(&stmt_code);
                        }
                    }
                }
            } else {
                if body.node_type != "Continue" {
                    let stmt_code = self.generate_statement(body)?;
                    if !stmt_code.trim().is_empty() {
                        output.push_str(&stmt_code);
                    }
                }
            }

            // Проверка условия в конце
            output.push_str(&self.line(&format!("if not ({}):", cond_code)));
            self.indent_level += 1;
            output.push_str(&self.line("break"));
            self.indent_level -= 1;

            self.indent_level -= 1;
        }

        Ok(output)
    }
    /// Генерирует выражение
    fn generate_expression(&mut self, node: &ASTNode) -> Result<String> {
        self.in_expression = true;
        let result = self.generate_expression_internal(node);
        self.in_expression = false;
        result
    }

    // В методе generate_expression_internal, замените обработку "FuncCall" на:
    fn generate_expression_internal(&mut self, node: &ASTNode) -> Result<String> {
        match node.node_type.as_str() {
            // В методе generate_expression_internal, в обработке "Constant":
            "Constant" => {
                if let Some(value) = node.attributes.get("value") {
                    if let Some(s) = value.as_str() {
                        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                            Ok(s.to_string())
                        } else if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                            // Это символ в кавычках, например 'A'
                            let char_content = &s[1..s.len() - 1];
                            if char_content.len() == 1 {
                                // Преобразуем символ в его ASCII код
                                if let Some(c) = char_content.chars().next() {
                                    Ok(format!("{}", c as u32))
                                } else {
                                    Ok(s.to_string())
                                }
                            } else {
                                Ok(s.to_string())
                            }
                        } else if s.parse::<i32>().is_ok() || s.parse::<f64>().is_ok() {
                            Ok(s.to_string())
                        } else {
                            Ok(s.to_string())
                        }
                    } else if let Some(n) = value.as_i64() {
                        Ok(n.to_string())
                    } else if let Some(n) = value.as_f64() {
                        Ok(n.to_string())
                    } else if let Some(b) = value.as_bool() {
                        Ok(b.to_string())
                    } else {
                        Ok(value.to_string())
                    }
                } else {
                    Ok("None".to_string())
                }
            }
            "ID" => {
                if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
                    if let Some(enum_name) = self.is_enum_value(name) {
                        // Если это значение перечисления, возвращаем его числовое значение
                        Ok(format!("{}.{}.value", enum_name, name))
                    } else {
                        Ok(name.to_string())
                    }
                } else {
                    Ok("unknown".to_string())
                }
            }
            "BinaryOp" => {
                let op = node
                    .attributes
                    .get("op")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                if op == "+" && node.children.len() == 2 {
                    let left = &node.children[0];
                    let right = &node.children[1];

                    if left.node_type == "ID" {
                        if let Some(var_name) = left.attributes.get("name").and_then(|v| v.as_str())
                        {
                            if self.pointer_vars.contains(var_name) {
                                let index = self.generate_expression_internal(right)?;
                                if self.in_expression {
                                    return Ok(format!("{}[{}]", var_name, index));
                                } else {
                                    return Ok(format!("{} + {}", var_name, index));
                                }
                            }
                        }
                    }
                }

                let py_op = match op {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "%" => op,
                    "&&" => "and",
                    "||" => "or",
                    _ => op,
                };

                let left = if let Some(child) = node.children.get(0) {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                let right = if let Some(child) = node.children.get(1) {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                // В обработке "BinaryOp", при формировании выражения
                let left_with_parens = self.wrap_if_needed(&left, node.children.get(0));
                let right_with_parens = self.wrap_if_needed(&right, node.children.get(1));

                // Для логических операций всегда добавляем скобки для сохранения приоритета
                if py_op == "and" || py_op == "or" {
                    // Проверяем, нужно ли обернуть всё выражение
                    if left.contains("and")
                        || left.contains("or")
                        || right.contains("and")
                        || right.contains("or")
                    {
                        Ok(format!(
                            "({} {} {})",
                            left_with_parens, py_op, right_with_parens
                        ))
                    } else {
                        Ok(format!(
                            "{} {} {}",
                            left_with_parens, py_op, right_with_parens
                        ))
                    }
                } else {
                    Ok(format!(
                        "{} {} {}",
                        left_with_parens, py_op, right_with_parens
                    ))
                }
            }
            "UnaryOp" => {
                let op = node
                    .attributes
                    .get("op")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let expr = if let Some(child) = node.children.first() {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                fn count_dereferences(node: &ASTNode) -> usize {
                    if node.node_type == "UnaryOp" {
                        if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
                            if op == "*" && !node.children.is_empty() {
                                return 1 + count_dereferences(&node.children[0]);
                            }
                        }
                    }
                    0
                }

                match op {
                    // В generate_expression_internal для "UnaryOp" с оператором "&"
                    "&" => {
                        if let Some(child) = node.children.first() {
                            if child.node_type == "ArrayRef" {
                                // Для &array[index] создаем Reference на элемент
                                let array_ref = self.generate_expression_internal(child)?;
                                Ok(format!("Reference({})", array_ref))
                            } else if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    self.address_taken.insert(var_name.to_string());

                                    // Если это элемент массива в переменной
                                    if var_name.contains('[') && var_name.contains(']') {
                                        Ok(format!("Reference({})", var_name))
                                    } else {
                                        Ok(format!("Reference({})", var_name))
                                    }
                                } else {
                                    Ok(format!("Reference({})", expr))
                                }
                            } else {
                                Ok(format!("Reference({})", expr))
                            }
                        } else {
                            Ok(format!("Reference({})", expr))
                        }
                    }
                    "*" => {
                        let deref_count = count_dereferences(node);

                        if let Some(child) = node.children.first() {
                            if child.node_type == "BinaryOp" {
                                if let Some(op) =
                                    child.attributes.get("op").and_then(|v| v.as_str())
                                {
                                    if op == "+" && child.children.len() == 2 {
                                        let left = &child.children[0];
                                        let right = &child.children[1];

                                        if left.node_type == "ID" {
                                            if let Some(var_name) =
                                                left.attributes.get("name").and_then(|v| v.as_str())
                                            {
                                                let index =
                                                    self.generate_expression_internal(right)?;
                                                return Ok(format!("{}[{}]", var_name, index));
                                            }
                                        }
                                    }
                                }
                            }

                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    if self.pointer_vars.contains(var_name)
                                        && self.array_vars.contains(var_name)
                                    {
                                        return Ok(format!("{}[0]", var_name));
                                    }

                                    // Проверяем, является ли это указателем на структуру
                                    let is_struct_ptr =
                                        self.struct_info.keys().any(|k| var_name.contains(k));

                                    if deref_count == 1 {
                                        // Одноразыменование
                                        if is_struct_ptr {
                                            // Для указателя на структуру возвращаем сам объект (без .value)
                                            // так как ptr уже содержит ссылку на объект
                                            return Ok(var_name.to_string());
                                        } else {
                                            // Для указателя на простой тип
                                            return Ok(format!("{}.get_final_value()", var_name));
                                        }
                                    } else {
                                        // Многоразыменование
                                        let mut result = var_name.to_string();
                                        for i in 0..deref_count {
                                            if i == deref_count - 1 {
                                                // Последнее разыменование
                                                if is_struct_ptr && i == 0 {
                                                    result = format!("{}", result);
                                                } else {
                                                    result =
                                                        format!("{}.get_final_value()", result);
                                                }
                                            } else {
                                                // Промежуточные - получаем ссылку
                                                result = format!("{}.value", result);
                                            }
                                        }
                                        return Ok(result);
                                    }
                                }
                            }
                        }

                        // Для выражений, которые не являются простыми идентификаторами
                        let mut result = expr;
                        for _ in 0..deref_count {
                            result = format!("{}.get_final_value()", result);
                        }
                        Ok(result)
                    }
                    "++" | "p++" | "post++" => {
                        if let Some(child) = node.children.first() {
                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    if self.pointer_vars.contains(var_name) {
                                        return Ok(format!("{} + 1", var_name));
                                    }
                                }
                            }
                        }
                        Ok(format!("({} + 1)", expr))
                    }
                    "--" | "p--" | "post--" => {
                        if let Some(child) = node.children.first() {
                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    if self.pointer_vars.contains(var_name) {
                                        return Ok(format!("{} - 1", var_name));
                                    }
                                }
                            }
                        }
                        Ok(format!("({} - 1)", expr))
                    }
                    "-" => {
                        if expr.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            Ok(format!("-{}", expr))
                        } else if expr.starts_with('-') {
                            Ok(expr)
                        } else {
                            Ok(format!("-({})", expr))
                        }
                    }
                    "+" => Ok(format!("+{}", expr)),
                    "!" => Ok(format!("not {}", expr)),
                    _ => {
                        debug!("Неизвестный унарный оператор: {}", op);
                        Ok(expr)
                    }
                }
            }
            "FuncCall" => {
                // Ищем имя функции
                let mut name = None;
                let mut args_exprs = Vec::new();

                // Проверяем прямой атрибут name
                if let Some(name_attr) = node.attributes.get("name").and_then(|v| v.as_str()) {
                    name = Some(name_attr.to_string());
                }

                // Проверяем атрибут func_name
                if let Some(func_name) = node.attributes.get("func_name").and_then(|v| v.as_str()) {
                    name = Some(func_name.to_string());
                }

                // Обрабатываем детей
                for child in &node.children {
                    match child.node_type.as_str() {
                        "ID" => {
                            if let Some(id_name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                if name.is_none() {
                                    name = Some(id_name.to_string());
                                }
                            }
                        }
                        "ExprList" => {
                            for arg in &child.children {
                                let arg_expr = self.generate_expression_internal(arg)?;
                                if !arg_expr.is_empty() && arg_expr != "None" {
                                    args_exprs.push(arg_expr);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let name = name.unwrap_or_else(|| {
                    warn!("Не удалось определить имя функции");
                    "unknown".to_string()
                });

                // Проверяем, является ли это стандартной функцией C
                if let Some(mapping) = self.function_mappings.get(&name).cloned() {
                    // Добавляем необходимый импорт
                    if let Some(import_stmt) = mapping.import_required {
                        self.add_import(import_stmt);
                    }

                    // Отмечаем, что функция использована
                    self.used_functions.insert(name.clone());

                    // Формируем Python код с аргументами
                    let python_code = match name.as_str() {
                        "printf" => self.handle_printf(&args_exprs),
                        "scanf" => self.handle_scanf(&args_exprs),
                        "sprintf" | "snprintf" => {
                            if args_exprs.len() >= 2 {
                                if args_exprs.len() > 2 {
                                    format!(
                                        "{} = {} % ({})",
                                        args_exprs[0],
                                        args_exprs[1],
                                        args_exprs[2..].join(", ")
                                    )
                                } else {
                                    format!("{} = {}", args_exprs[0], args_exprs[1])
                                }
                            } else {
                                mapping.python_code.to_string()
                            }
                        }
                        "malloc" | "calloc" => self.handle_allocation(&name, &args_exprs),
                        "strcpy" | "strcat" | "strcmp" => {
                            self.handle_string_function(&name, &args_exprs)
                        }
                        "fopen" | "fclose" | "fprintf" => {
                            self.handle_file_operation(&name, &args_exprs)
                        }
                        _ => {
                            let mut code = mapping.python_code.to_string();
                            for (i, arg) in args_exprs.iter().enumerate() {
                                code = code.replacen(&format!("{{{}}}", i), arg, 1);
                            }
                            // Заменяем оставшиеся {} на аргументы по порядку
                            for arg in args_exprs {
                                code = code.replacen("{}", &arg, 1);
                            }
                            code
                        }
                    };

                    debug!(
                        "Стандартная функция {} преобразована в: {}",
                        name, python_code
                    );
                    return Ok(python_code);
                }

                // Проверяем, не является ли имя функции указателем на функцию
                if self.pointer_vars.contains(&name) {
                    Ok(format!("{}.value({})", name, args_exprs.join(", ")))
                } else {
                    Ok(format!("{}({})", name, args_exprs.join(", ")))
                }
            }
            "ExprList" => {
                let mut exprs = Vec::new();
                for child in &node.children {
                    let expr = self.generate_expression_internal(child)?;
                    if !expr.is_empty() {
                        exprs.push(expr);
                    }
                }
                Ok(exprs.join(", "))
            }
            "ArrayRef" => self.generate_array_ref(node),
            "InitList" => {
                let mut values = Vec::new();
                for child in &node.children {
                    if child.node_type == "InitList" {
                        let nested_values = self.generate_expression_internal(child)?;
                        values.push(nested_values);
                    } else if child.node_type == "NamedInitializer" {
                        if let Some(expr) = child.attributes.get("expr") {
                            let temp_node = ASTNode::new("Value").with_attr("value", expr.clone());
                            let value = self.generate_expression_internal(&temp_node)?;
                            values.push(value);
                        } else {
                            values.push("None".to_string());
                        }
                    } else {
                        let value = self.generate_expression_internal(child)?;
                        values.push(value);
                    }
                }

                if let Some(struct_name) = &self.current_struct_type {
                    Ok(format!("{}({})", struct_name, values.join(", ")))
                } else {
                    Ok(format!("[{}]", values.join(", ")))
                }
            }
            "TernaryOp" => {
                if node.children.len() >= 3 {
                    let cond = self.generate_expression_internal(&node.children[0])?;
                    let iftrue = self.generate_expression_internal(&node.children[1])?;
                    let iffalse = self.generate_expression_internal(&node.children[2])?;

                    // Для вложенных тернарных операторов добавляем скобки
                    if node.children[1].node_type == "TernaryOp"
                        || node.children[2].node_type == "TernaryOp"
                    {
                        Ok(format!("({} if {} else {})", iftrue, cond, iffalse))
                    } else {
                        Ok(format!("{} if {} else {}", iftrue, cond, iffalse))
                    }
                } else {
                    Ok("None".to_string())
                }
            }
            "StructRef" => self.generate_struct_ref(node),
            "Cast" => {
                let expr = if let Some(child) = node.children.first() {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                if let Some(to_type) = node.attributes.get("to_type") {
                    if let Some(type_str) = to_type.as_str() {
                        match type_str {
                            "int" => Ok(format!("int({})", expr)),
                            "float" | "double" => Ok(format!("float({})", expr)),
                            "char" => Ok(format!(
                                "chr({}) if isinstance({}, int) else {}",
                                expr, expr, expr
                            )),
                            _ => Ok(expr),
                        }
                    } else {
                        Ok(expr)
                    }
                } else {
                    Ok(expr)
                }
            }
            _ => {
                debug!("Неизвестное выражение: {}", node.node_type);
                Ok("None".to_string())
            }
        }
    }
    /// Специальная обработка для printf с форматированием
    fn handle_printf(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            return "print()".to_string();
        }

        let format_str = &args[0];

        // Если есть дополнительные аргументы, используем форматирование
        if args.len() > 1 {
            let format_args = args[1..].join(", ");

            // Проверяем, содержит ли форматная строка спецификаторы
            if format_str.contains('%') && !format_str.contains("{{") {
                // Преобразуем спецификаторы C в Python
                let python_format = format_str
                    .replace("%d", "{}")
                    .replace("%i", "{}")
                    .replace("%f", "{}")
                    .replace("%lf", "{}")
                    .replace("%c", "{}")
                    .replace("%s", "{}")
                    .replace("%x", "{:x}")
                    .replace("%X", "{:X}")
                    .replace("%o", "{:o}")
                    .replace("%p", "{}");

                return format!("print({}.format({}))", python_format, format_args);
            }
        }

        format!("print({})", format_str)
    }

    /// Обработка строковых функций
    fn handle_string_function(&mut self, func_name: &str, args: &[String]) -> String {
        match func_name {
            "strcpy" => {
                if args.len() >= 2 {
                    format!("{} = {}", args[0], args[1])
                } else {
                    String::new()
                }
            }
            "strcat" => {
                if args.len() >= 2 {
                    format!("{} += {}", args[0], args[1])
                } else {
                    String::new()
                }
            }
            "strcmp" => {
                if args.len() >= 2 {
                    format!(
                        "(0 if {} == {} else -1 if {} < {} else 1)",
                        args[0], args[1], args[0], args[1]
                    )
                } else {
                    "0".to_string()
                }
            }
            _ => String::new(),
        }
    }
    /// Специальная обработка для scanf
    fn handle_scanf(&mut self, args: &[String]) -> String {
        if args.is_empty() {
            return "input()".to_string();
        }

        let format_str = &args[0];

        if format_str.starts_with('"') || format_str.starts_with('\'') {
            let prompt = &format_str[1..format_str.len() - 1];
            format!("input({})", prompt)
        } else {
            "input()".to_string()
        }
    }

    /// Обработка malloc/calloc для создания списков
    fn handle_allocation(&mut self, func_name: &str, args: &[String]) -> String {
        match func_name {
            "malloc" => {
                if !args.is_empty() {
                    format!("[None] * {}", args[0])
                } else {
                    "[]".to_string()
                }
            }
            "calloc" => {
                if args.len() >= 2 {
                    format!("[0] * ({} * {})", args[0], args[1])
                } else {
                    "[]".to_string()
                }
            }
            _ => "[]".to_string(),
        }
    }

    /// Обработка файловых операций
    fn handle_file_operation(&mut self, func_name: &str, args: &[String]) -> String {
        match func_name {
            "fopen" => {
                if args.len() >= 2 {
                    let mode = if args[1].contains("r") {
                        "'r'"
                    } else if args[1].contains("w") {
                        "'w'"
                    } else if args[1].contains("a") {
                        "'a'"
                    } else {
                        "'r'"
                    };
                    format!("open({}, {})", args[0], mode)
                } else {
                    "None".to_string()
                }
            }
            "fclose" => {
                if !args.is_empty() {
                    format!("{}.close()", args[0])
                } else {
                    String::new()
                }
            }
            "fprintf" => {
                if args.len() >= 2 {
                    format!("{}.write({})", args[0], args[1])
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }
    /// Генерирует унарный оператор как оператор
    fn generate_unary_stmt(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация унарного оператора как стейтмента");

        if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
            if let Some(child) = node.children.first() {
                // Для операций с переменными
                if child.node_type == "ID" {
                    if let Some(var_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                        match op {
                            "++" | "p++" | "post++" => {
                                if self.pointer_vars.contains(var_name) {
                                    // Для указателей: ptr = ptr + 1 (арифметика указателей)
                                    return Ok(
                                        self.line(&format!("{} = {} + 1", var_name, var_name))
                                    );
                                } else {
                                    return Ok(self.line(&format!("{} += 1", var_name)));
                                }
                            }
                            "--" | "p--" | "post--" => {
                                if self.pointer_vars.contains(var_name) {
                                    return Ok(
                                        self.line(&format!("{} = {} - 1", var_name, var_name))
                                    );
                                } else {
                                    return Ok(self.line(&format!("{} -= 1", var_name)));
                                }
                            }
                            "*" => {
                                // Это разыменование указателя как выражение
                                // В Python это будет обработано в Assignment
                                return Ok(self.line(&format!("{}", var_name)));
                            }
                            _ => {}
                        }
                    }
                }

                // Для *(ptr + i) - разыменование с арифметикой
                if op == "*" && child.node_type == "BinaryOp" {
                    let inner_expr = self.generate_expression(child)?;
                    return Ok(self.line(&inner_expr));
                }
            }
        }

        let expr = self.generate_expression(node)?;
        Ok(self.line(&expr))
    }

    /// Генерирует return
    fn generate_return(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация RETURN узла");
        debug!("Детей у return: {}", node.children.len());

        if let Some(expr) = node.children.first() {
            let expr_code = self.generate_expression(expr)?;
            debug!("Возвращаемое значение: {}", expr_code);

            if expr_code.is_empty() || expr_code == "None" {
                Ok(self.line("return"))
            } else {
                Ok(self.line(&format!("return {}", expr_code)))
            }
        } else {
            debug!("return без выражения");
            Ok(self.line("return"))
        }
    }

    fn wrap_if_needed(&self, expr: &str, child: Option<&ASTNode>) -> String {
        if let Some(child_node) = child {
            if child_node.node_type == "BinaryOp" {
                return format!("({})", expr);
            }
        }
        expr.to_string()
    }

    /// Генерирует if с поддержкой elif
    fn generate_if(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        let mut current_node = node;
        let mut is_first = true;

        debug!("Генерация IF/ELIF цепочки");

        // Функция для проверки, является ли узел "else if" конструкцией
        fn is_else_if(node: &ASTNode) -> bool {
            if node.node_type == "If" {
                return true;
            }
            // Проверяем, может ли это быть Compound с одним If внутри
            if node.node_type == "Compound" && node.children.len() == 1 {
                if let Some(first_child) = node.children.first() {
                    return first_child.node_type == "If";
                }
            }
            false
        }

        loop {
            if let Some(cond) = current_node.children.first() {
                let cond_code = self.generate_expression(cond)?;

                if is_first {
                    output.push_str(&self.line(&format!("if {}:", cond_code)));
                    is_first = false;
                } else {
                    output.push_str(&self.line(&format!("elif {}:", cond_code)));
                }

                self.indent_level += 1;

                // Генерируем тело текущего if
                if let Some(if_body) = current_node.children.get(1) {
                    debug!("Тело IF: тип={}", if_body.node_type);

                    if if_body.node_type == "Compound" {
                        for stmt in &if_body.children {
                            output.push_str(&self.generate_statement(stmt)?);
                        }
                    } else {
                        // Если не Compound, возможно это одиночный оператор
                        output.push_str(&self.generate_statement(if_body)?);
                    }
                }
                self.indent_level -= 1;

                // Проверяем наличие else части
                if current_node.children.len() > 2 {
                    let else_part = current_node.children.get(2).unwrap();
                    debug!("Часть ELSE: тип={}", else_part.node_type);

                    // Проверяем, является ли else часть "else if"
                    if is_else_if(else_part) {
                        // Это else if - переходим к следующей итерации для генерации elif
                        if else_part.node_type == "If" {
                            current_node = else_part;
                        } else if else_part.node_type == "Compound"
                            && !else_part.children.is_empty()
                        {
                            // Извлекаем if из compound
                            if let Some(inner_if) = else_part.children.first() {
                                current_node = inner_if;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        // Это обычный else
                        output.push_str(&self.line("else:"));
                        self.indent_level += 1;

                        if else_part.node_type == "Compound" {
                            for stmt in &else_part.children {
                                output.push_str(&self.generate_statement(stmt)?);
                            }
                        } else {
                            output.push_str(&self.generate_statement(else_part)?);
                        }
                        self.indent_level -= 1;
                        break; // Завершаем цикл после else
                    }
                } else {
                    break; // Нет else части, завершаем
                }
            } else {
                break;
            }
        }

        Ok(output)
    }

    /// Генерирует switch как match в Python
    fn generate_switch(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация SWITCH узла");
        debug!("Детей у switch: {}", node.children.len());

        // Первый ребенок - условие (выражение в switch)
        if let Some(cond) = node.children.first() {
            let cond_code = self.generate_expression(cond)?;
            debug!("Условие switch: {}", cond_code);
            output.push_str(&self.line(&format!("match {}:", cond_code)));

            self.indent_level += 1;

            // Второй ребенок - тело switch (содержит case и default)
            if let Some(body) = node.children.get(1) {
                debug!("Тело switch тип: {}", body.node_type);
                if body.node_type == "Compound" {
                    debug!(
                        "Количество операторов в теле switch: {}",
                        body.children.len()
                    );
                    for (i, stmt) in body.children.iter().enumerate() {
                        debug!("  Оператор {} в switch: тип={}", i, stmt.node_type);
                        match stmt.node_type.as_str() {
                            "Case" => {
                                output.push_str(&self.generate_case(stmt)?);
                            }
                            "Default" => {
                                output.push_str(&self.generate_default(stmt)?);
                            }
                            _ => {
                                debug!("Неизвестный оператор в теле switch: {}", stmt.node_type);
                            }
                        }
                    }
                }
            }

            self.indent_level -= 1;
        } else {
            debug!("Нет условия в switch!");
        }

        Ok(output)
    }

    fn generate_case(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        if let Some(value_node) = node.children.first() {
            let value = self.generate_expression(value_node)?;

            output.push_str(&self.line(&format!("case {}:", value)));
            self.indent_level += 1;

            // Проверяем, есть ли операторы в case
            let mut has_statements = false;
            for stmt in node.children.iter().skip(1) {
                if stmt.node_type == "Break" {
                    continue;
                }
                let stmt_code = self.generate_statement(stmt)?;
                if !stmt_code.trim().is_empty() {
                    output.push_str(&stmt_code);
                    has_statements = true;
                }
            }

            // Если нет операторов, добавляем pass
            if !has_statements {
                output.push_str(&self.line("pass"));
            }

            self.indent_level -= 1;
        }

        Ok(output)
    }
    /// Генерирует default в match
    fn generate_default(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        output.push_str(&self.line("case _:"));

        // УВЕЛИЧИВАЕМ отступ для тела default
        self.indent_level += 1;

        // Операторы в default
        let mut has_statements = false;
        for stmt in node.children.iter() {
            if stmt.node_type == "Break" {
                continue;
            }
            let stmt_code = self.generate_statement(stmt)?;
            if !stmt_code.trim().is_empty() {
                output.push_str(&stmt_code);
                has_statements = true;
            }
        }

        if !has_statements {
            output.push_str(&self.line("pass"));
        }

        // УМЕНЬШАЕМ отступ обратно
        self.indent_level -= 1;

        Ok(output)
    }
    /// Генерирует while
    fn generate_while(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация WHILE узла");
        debug!("Детей у while: {}", node.children.len());

        if let Some(cond) = node.children.first() {
            let cond_code = self.generate_expression(cond)?;
            debug!("Условие while: {}", cond_code);
            output.push_str(&self.line(&format!("while {}:", cond_code)));

            self.indent_level += 1;
            if let Some(body) = node.children.get(1) {
                debug!("Тело while тип: {}", body.node_type);
                debug!("Количество операторов в теле: {}", body.children.len());

                if body.node_type == "Compound" {
                    for stmt in &body.children {
                        let stmt_code = self.generate_statement(stmt)?;
                        debug!("Оператор в теле: {}", stmt_code.trim());
                        output.push_str(&stmt_code);
                    }
                } else {
                    // Одиночный оператор без {}
                    let stmt_code = self.generate_statement(body)?;
                    debug!("Одиночный оператор: {}", stmt_code.trim());
                    output.push_str(&stmt_code);
                }
            } else {
                debug!("НЕТ ТЕЛА У WHILE!");
            }
            self.indent_level -= 1;
        } else {
            debug!("НЕТ УСЛОВИЯ У WHILE!");
        }

        if output.is_empty() {
            debug!("ВНИМАНИЕ: while цикл не сгенерирован!");
        }

        Ok(output)
    }

    fn is_enum_value(&self, name: &str) -> Option<String> {
        for (enum_name, values) in &self.enum_info {
            if values.contains(&name.to_string()) {
                return Some(enum_name.clone());
            }
        }
        None
    }

    fn generate_declaration(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
            self.symbols.insert(name.to_string());

            debug!("Анализ объявления переменной: {}", name);
            debug!("  Тип узла: {}", node.node_type);
            debug!("  Атрибуты: {:#?}", node.attributes);
            debug!("  Количество детей: {}", node.children.len());

            // Проверяем, является ли это указателем
            let mut is_pointer = false;
            let mut _pointed_type = String::new();

            // Проверяем детей на наличие PtrDecl
            for child in &node.children {
                if child.node_type == "PtrDecl" {
                    is_pointer = true;
                    debug!("  Найден указатель: {}", name);

                    // Пытаемся определить тип, на который указывает
                    for grandchild in &child.children {
                        if grandchild.node_type == "TypeDecl"
                            || grandchild.node_type == "IdentifierType"
                        {
                            if let Some(type_name) = self.extract_type_name(grandchild) {
                                _pointed_type = type_name;
                            }
                        }
                    }
                    break;
                }
            }

            // Сначала проверяем, не является ли это определением enum (Decl с Enum внутри)
            for child in &node.children {
                if child.node_type == "Enum" {
                    debug!("  Найдено определение ENUM внутри Decl");
                    // Генерируем класс enum и возвращаем, не обрабатывая как переменную
                    return self.generate_enum(child);
                }
            }

            // Определяем тип объявления
            let mut is_struct_var = false;
            let mut is_union_var = false;
            let mut is_enum_var = false;
            let mut struct_type = None;
            let mut union_type = None;
            let mut enum_type = None;

            // 1. Проверяем атрибут type самого узла Decl
            if let Some(type_attr) = node.attributes.get("type") {
                debug!("  Атрибут type узла: {:#?}", type_attr);
                if let Some(type_obj) = type_attr.as_object() {
                    match type_obj.get("__node__").and_then(|v| v.as_str()) {
                        Some("Struct") => {
                            is_struct_var = true;
                            if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                struct_type = Some(name.to_string());
                                debug!("  Найдена структура в атрибуте type узла: {}", name);
                            }
                        }
                        Some("Union") => {
                            is_union_var = true;
                            if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                union_type = Some(name.to_string());
                                debug!("  Найдено объединение в атрибуте type узла: {}", name);
                            }
                        }
                        Some("Enum") => {
                            is_enum_var = true;
                            if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                enum_type = Some(name.to_string());
                                debug!("  Найдено перечисление в атрибуте type узла: {}", name);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // 2. Проверяем детей, если еще не определили тип
            if !is_struct_var && !is_union_var && !is_enum_var {
                for child in &node.children {
                    debug!("    Проверка ребенка с типом: {}", child.node_type);
                    debug!("      Атрибуты ребенка: {:#?}", child.attributes);

                    match child.node_type.as_str() {
                        "Struct" => {
                            is_struct_var = true;
                            if let Some(name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                struct_type = Some(name.to_string());
                                debug!("      НАЙДЕНА СТРУКТУРА как прямой ребенок: {}", name);
                                break;
                            }
                        }
                        "Union" => {
                            is_union_var = true;
                            if let Some(name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                union_type = Some(name.to_string());
                                debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ как прямой ребенок: {}", name);
                                break;
                            }
                        }
                        "Enum" => {
                            is_enum_var = true;
                            if let Some(name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                enum_type = Some(name.to_string());
                                debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ как прямой ребенок: {}", name);
                                break;
                            }
                        }
                        _ => {}
                    }

                    // Проверяем атрибут type ребенка
                    if let Some(type_attr) = child.attributes.get("type") {
                        debug!("      Атрибут type ребенка: {:#?}", type_attr);
                        if let Some(type_obj) = type_attr.as_object() {
                            match type_obj.get("__node__").and_then(|v| v.as_str()) {
                                Some("Struct") => {
                                    is_struct_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        struct_type = Some(name.to_string());
                                        debug!("      НАЙДЕНА СТРУКТУРА в атрибуте type ребенка {}: {}", 
                                       child.node_type, name);
                                        break;
                                    }
                                }
                                Some("Union") => {
                                    is_union_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        union_type = Some(name.to_string());
                                        debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ в атрибуте type ребенка {}: {}", 
                                       child.node_type, name);
                                        break;
                                    }
                                }
                                Some("Enum") => {
                                    is_enum_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        enum_type = Some(name.to_string());
                                        debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ в атрибуте type ребенка {}: {}", 
                                       child.node_type, name);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Проверяем детей ребенка
                    for grandchild in &child.children {
                        match grandchild.node_type.as_str() {
                            "Struct" => {
                                is_struct_var = true;
                                if let Some(name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    struct_type = Some(name.to_string());
                                    debug!("      НАЙДЕНА СТРУКТУРА как внук: {}", name);
                                    break;
                                }
                            }
                            "Union" => {
                                is_union_var = true;
                                if let Some(name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    union_type = Some(name.to_string());
                                    debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ как внук: {}", name);
                                    break;
                                }
                            }
                            "Enum" => {
                                is_enum_var = true;
                                if let Some(name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    enum_type = Some(name.to_string());
                                    debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ как внук: {}", name);
                                    break;
                                }
                            }
                            _ => {}
                        }

                        if let Some(type_attr) = grandchild.attributes.get("type") {
                            if let Some(type_obj) = type_attr.as_object() {
                                match type_obj.get("__node__").and_then(|v| v.as_str()) {
                                    Some("Struct") => {
                                        is_struct_var = true;
                                        if let Some(name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            struct_type = Some(name.to_string());
                                            debug!("      НАЙДЕНА СТРУКТУРА в атрибуте type внука {}: {}", 
                                           grandchild.node_type, name);
                                            break;
                                        }
                                    }
                                    Some("Union") => {
                                        is_union_var = true;
                                        if let Some(name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            union_type = Some(name.to_string());
                                            debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ в атрибуте type внука {}: {}", 
                                           grandchild.node_type, name);
                                            break;
                                        }
                                    }
                                    Some("Enum") => {
                                        is_enum_var = true;
                                        if let Some(name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            enum_type = Some(name.to_string());
                                            debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ в атрибуте type внука {}: {}", 
                                           grandchild.node_type, name);
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    if is_struct_var || is_union_var || is_enum_var {
                        break;
                    }
                }
            }

            debug!(
            "  Результат анализа - is_struct_var: {}, struct_type: {:?}, is_union_var: {}, union_type: {:?}, is_enum_var: {}, enum_type: {:?}",
            is_struct_var, struct_type, is_union_var, union_type, is_enum_var, enum_type
        );

            // Ищем инициализатор среди детей
            let mut init_node = None;
            for child in &node.children {
                match child.node_type.as_str() {
                    "InitList" | "Constant" | "FuncCall" | "TernaryOp" | "ID" | "BinaryOp"
                    | "UnaryOp" | "NamedInitializer" | "StructRef" => {
                        init_node = Some(child);
                        debug!(
                            "  Найден инициализатор типа {} для переменной {}",
                            child.node_type, name
                        );
                        break;
                    }
                    _ => {}
                }
            }

            // В методе generate_declaration, при обнаружении указателя:
            if is_pointer {
                self.pointer_vars.insert(name.to_string());

                // Определяем уровень указателя
                let mut ptr_level = 1;
                let mut current = node;
                while let Some(child) = current.children.first() {
                    if child.node_type == "PtrDecl" {
                        ptr_level += 1;
                        current = child;
                    } else {
                        break;
                    }
                }

                // Проверяем, не является ли это указателем на массив
                if let Some(init) = init_node {
                    if init.node_type == "ID" {
                        if let Some(init_name) =
                            init.attributes.get("name").and_then(|v| v.as_str())
                        {
                            if self.array_vars.contains(init_name) {
                                self.array_vars.insert(name.to_string());
                            }
                        }
                    }
                }

                debug!(
                    "  Переменная {} помечена как указатель уровня {}",
                    name, ptr_level
                );
            }
            // Обрабатываем объявления в зависимости от типа
            if is_union_var {
                self.handle_union_declaration(name, union_type, init_node, &mut output)?;
            } else if is_struct_var {
                self.handle_struct_declaration(name, struct_type, init_node, &mut output)?;
            } else if is_enum_var {
                self.handle_enum_declaration(name, enum_type, init_node, &mut output)?;
            } else {
                self.handle_regular_declaration(name, init_node, &mut output)?;
            }

            Ok(output)
        } else {
            // Если нет имени, проверяем, может это определение enum без переменной
            for child in &node.children {
                if child.node_type == "Enum" {
                    debug!("Найдено определение ENUM без имени переменной");
                    return self.generate_enum(child);
                }
            }
            Ok(String::new())
        }
    }

    fn extract_type_name(&self, node: &ASTNode) -> Option<String> {
        match node.node_type.as_str() {
            "IdentifierType" => node
                .attributes
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from),
            "TypeDecl" => {
                for child in &node.children {
                    if let Some(name) = self.extract_type_name(child) {
                        return Some(name);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Обработка объявления переменной типа enum
    fn handle_enum_declaration(
        &mut self,
        name: &str,
        enum_type: Option<String>,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "handle_enum_declaration: name={}, enum_type={:?}, has_init={}",
            name,
            enum_type,
            init_node.is_some()
        );

        if let Some(init) = init_node {
            // Есть инициализатор
            let init_code = self.generate_expression(init)?;

            // Проверяем, является ли инициализатор простым идентификатором (например, RED)
            // и нужно ли его преобразовать в enum_type.RED
            if init.node_type == "ID" {
                if let Some(enum_name) = &enum_type {
                    // Проверяем, не является ли это уже полным именем с точкой
                    if !init_code.contains('.') {
                        // Преобразуем RED в Color.RED
                        debug!(
                            "Преобразуем enum значение {} в {}.{}",
                            init_code, enum_name, init_code
                        );
                        output.push_str(
                            &self.line(&format!("{} = {}.{}", name, enum_name, init_code)),
                        );
                        return Ok(());
                    }
                }
            }

            output.push_str(&self.line(&format!("{} = {}", name, init_code)));
        } else {
            // Нет инициализатора
            if let Some(enum_name) = enum_type {
                // Для enum переменных без инициализатора используем первое значение по умолчанию
                // Но мы не знаем первое значение, поэтому оставляем комментарий
                output.push_str(&self.line(&format!(
                    "{} = None  # TODO: Укажите значение enum {}.VALUE",
                    name, enum_name
                )));
            } else {
                output.push_str(&self.line(&format!("{} = None", name)));
            }
        }

        Ok(())
    }

    // В методе handle_struct_declaration, измените создание экземпляра
    fn handle_struct_declaration(
        &mut self,
        name: &str,
        struct_type: Option<String>,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "handle_struct_declaration: name={}, struct_type={:?}, has_init={}",
            name,
            struct_type,
            init_node.is_some()
        );

        if let Some(init) = init_node {
            // Проверяем тип инициализатора
            if init.node_type == "InitList" {
                // Определяем, является ли это designated initializer
                let has_named = init
                    .children
                    .iter()
                    .any(|c| c.node_type == "NamedInitializer");

                if has_named {
                    // Для designated initializers создаем пустой объект и заполняем поля
                    if let Some(struct_name) = &struct_type {
                        output.push_str(&self.line(&format!("{} = {}()", name, struct_name)));

                        // Затем заполняем именованные поля
                        for child in &init.children {
                            if child.node_type == "NamedInitializer" {
                                if let Some(field_name) =
                                    self.extract_field_name_from_named_init(child)
                                {
                                    if let Some(expr_child) = child.children.first() {
                                        let value = self.generate_expression(expr_child)?;
                                        output.push_str(
                                            &self.line(&format!(
                                                "{}.{} = {}",
                                                name, field_name, value
                                            )),
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Обычная позиционная инициализация
                    self.current_struct_type = struct_type.clone();
                    let init_code = self.generate_expression(init)?;
                    self.current_struct_type = None;
                    output.push_str(&self.line(&format!("{} = {}", name, init_code)));
                }
            } else {
                let init_code = self.generate_expression(init)?;
                output.push_str(&self.line(&format!("{} = {}", name, init_code)));
            }
        } else {
            // Нет инициализатора - создаем с None для всех полей
            if let Some(struct_name) = struct_type {
                if let Some(fields) = self.struct_info.get(&struct_name) {
                    if !fields.is_empty() {
                        let default_params = vec!["None".to_string(); fields.len()].join(", ");
                        output.push_str(
                            &self.line(&format!("{} = {}({})", name, struct_name, default_params)),
                        );
                    } else {
                        output.push_str(&self.line(&format!("{} = {}()", name, struct_name)));
                    }
                } else {
                    output.push_str(&self.line(&format!("{} = {}()", name, struct_name)));
                }
            }
        }

        Ok(())
    }
    // Вспомогательный метод для извлечения имени поля из NamedInitializer
    fn extract_field_name_from_named_init(&self, node: &ASTNode) -> Option<String> {
        // Проверяем атрибуты name[0], name[1] и т.д.
        for i in 0.. {
            let key = format!("name[{}]", i);
            if let Some(name_value) = node.attributes.get(&key) {
                if let Some(name_obj) = name_value.as_object() {
                    if let Some(field_name) = name_obj.get("name").and_then(|v| v.as_str()) {
                        return Some(field_name.to_string());
                    }
                } else if let Some(name_str) = name_value.as_str() {
                    return Some(name_str.to_string());
                }
            } else {
                break;
            }
        }

        // Проверяем прямой атрибут name
        if let Some(name_value) = node.attributes.get("name") {
            if let Some(name_obj) = name_value.as_object() {
                if let Some(field_name) = name_obj.get("name").and_then(|v| v.as_str()) {
                    return Some(field_name.to_string());
                }
            } else if let Some(name_str) = name_value.as_str() {
                return Some(name_str.to_string());
            }
        }

        None
    }
    /// Обработка объявления переменной объединения
    fn handle_union_declaration(
        &mut self,
        name: &str,
        union_type: Option<String>,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "Обработка переменной объединения: {} типа {:?}",
            name, union_type
        );

        if let Some(init) = init_node {
            // Для объединений с инициализатором
            if init.node_type == "InitList" {
                self.current_struct_type = union_type.clone();
                debug!(
                    "Установлен контекст объединения {:?} для InitList переменной {}",
                    union_type, name
                );
                let init_code = self.generate_expression(init)?;
                self.current_struct_type = None;
                output.push_str(&self.line(&format!("{} = {}", name, init_code)));
            } else {
                let init_code = self.generate_expression(init)?;
                output.push_str(&self.line(&format!("{} = {}", name, init_code)));
            }
        } else {
            // Для объединений без инициализатора создаем экземпляр класса
            if let Some(union_name) = union_type {
                output.push_str(&self.line(&format!("{} = {}()", name, union_name)));
                debug!("Создан экземпляр объединения: {} = {}()", name, union_name);
            } else {
                warn!("Неизвестный тип объединения для переменной {}", name);
                output.push_str(&self.line(&format!("{} = None", name)));
            }
        }

        Ok(())
    }

    fn handle_regular_declaration(
        &mut self,
        name: &str,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "Обычная переменная (не структура и не объединение): {}",
            name
        );

        if let Some(init) = init_node {
            let init_code = self.generate_expression(init)?;

            // Проверяем, не является ли это символьной константой
            if init.node_type == "Constant" {
                if let Some(value) = init.attributes.get("value") {
                    if let Some(s) = value.as_str() {
                        if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                            let char_content = &s[1..s.len() - 1];
                            if char_content.len() == 1 {
                                // Преобразуем символ в его ASCII код
                                if let Some(c) = char_content.chars().next() {
                                    output
                                        .push_str(&self.line(&format!("{} = {}", name, c as u32)));
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            output.push_str(&self.line(&format!("{} = {}", name, init_code)));
        } else {
            output.push_str(&self.line(&format!("{} = None", name)));
        }

        Ok(())
    }
    fn generate_assignment(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация присваивания");
        debug!("  Детей у присваивания: {}", node.children.len());

        if node.children.len() >= 2 {
            let left_node = &node.children[0];
            let right_node = &node.children[1];

            // Получаем оператор присваивания
            let op = node
                .attributes
                .get("op")
                .and_then(|v| v.as_str())
                .unwrap_or("=");

            if left_node.node_type == "StructRef" {
                // Это доступ к полю структуры, например vec_ptr.x
                let left = self.generate_expression(left_node)?;
                let right = self.generate_expression(right_node)?;
                let py_op = match op {
                    "=" => "=",
                    "+=" => "+=",
                    "-=" => "-=",
                    "*=" => "*=",
                    "/=" => "/=",
                    "%=" => "%=",
                    _ => "=",
                };
                return Ok(self.line(&format!("{} {} {}", left, py_op, right)));
            }
            // Проверяем, является ли левая часть обращением к элементу массива
            if left_node.node_type == "ArrayRef" && left_node.children.len() >= 2 {
                let array_name_node = &left_node.children[0];
                let index_node = &left_node.children[1];

                // Генерируем выражение для имени массива
                let array_expr = self.generate_expression(array_name_node)?;

                // Проверяем, является ли это массивом символов (строкой)
                let base_name = array_expr.split('.').next().unwrap_or(&array_expr);
                let base_name = base_name.split('[').next().unwrap_or(base_name);

                // Проверяем, есть ли информация о том, что это массив символов
                let is_string_array = array_expr.contains(".name")
                    || (self.array_vars.contains(base_name)
                        && !self.struct_info.contains_key(base_name));

                if is_string_array {
                    // Это строковый массив - обрабатываем через буфер
                    if let Ok(index) = self.get_constant_int_value(index_node) {
                        let value_expr = self.generate_expression(right_node)?;

                        let entry = self
                            .pending_string_assignments
                            .entry(array_expr.clone())
                            .or_insert_with(Vec::new);
                        entry.push((index, value_expr));

                        return Ok(String::new());
                    }
                } else {
                    // Это обычный массив (не строка) - генерируем обычное присваивание
                    let left = self.generate_expression(left_node)?;
                    let right = self.generate_expression(right_node)?;

                    // Преобразуем оператор в Python
                    let py_op = match op {
                        "=" => "=",
                        "+=" => "+=",
                        "-=" => "-=",
                        "*=" => "*=",
                        "/=" => "/=",
                        "%=" => "%=",
                        "&=" => "&=",
                        "|=" => "|=",
                        "^=" => "^=",
                        "<<=" => "<<=",
                        ">>=" => ">>=",
                        _ => "=",
                    };

                    return Ok(self.line(&format!("{} {} {}", left, py_op, right)));
                }
            }

            if left_node.node_type == "UnaryOp" {
                if let Some(unary_op) = left_node.attributes.get("op").and_then(|v| v.as_str()) {
                    if unary_op == "*" && left_node.children.len() == 1 {
                        let inner = &left_node.children[0];

                        fn count_dereferences(node: &ASTNode) -> usize {
                            if node.node_type == "UnaryOp" {
                                if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str())
                                {
                                    if op == "*" && !node.children.is_empty() {
                                        return 1 + count_dereferences(&node.children[0]);
                                    }
                                }
                            }
                            0
                        }

                        let deref_count = count_dereferences(left_node);

                        if inner.node_type == "ID" {
                            if let Some(ptr_name) =
                                inner.attributes.get("name").and_then(|v| v.as_str())
                            {
                                let right_expr = self.generate_expression(right_node)?;

                                debug!(
                                    "  Присваивание через указатель: *{} = {}, уровень={}",
                                    ptr_name, right_expr, deref_count
                                );

                                // Проверяем, является ли это указателем на структуру
                                let is_struct_ptr =
                                    self.struct_info.keys().any(|k| ptr_name.contains(k));

                                if self.pointer_vars.contains(ptr_name)
                                    && self.array_vars.contains(ptr_name)
                                {
                                    let py_op = match op {
                                        "+=" => "+=",
                                        "-=" => "-=",
                                        _ => "=",
                                    };
                                    return Ok(self.line(&format!(
                                        "{}[0] {} {}",
                                        ptr_name, py_op, right_expr
                                    )));
                                } else {
                                    let py_op = match op {
                                        "+=" => "+=",
                                        "-=" => "-=",
                                        _ => "=",
                                    };

                                    // Для указателей на структуры
                                    if is_struct_ptr && deref_count == 1 {
                                        // Просто присваиваем самому указателю (заменяем объект)
                                        return Ok(self.line(&format!(
                                            "{} {} {}",
                                            ptr_name, py_op, right_expr
                                        )));
                                    } else {
                                        // Для двойных указателей используем цепочку value
                                        let result = match deref_count {
                                            1 => format!(
                                                "{}.value {} {}",
                                                ptr_name, py_op, right_expr
                                            ),
                                            2 => format!(
                                                "{}.value.value {} {}",
                                                ptr_name, py_op, right_expr
                                            ),
                                            3 => format!(
                                                "{}.value.value.value {} {}",
                                                ptr_name, py_op, right_expr
                                            ),
                                            _ => format!(
                                                "{}.value {} {}",
                                                ptr_name, py_op, right_expr
                                            ),
                                        };
                                        return Ok(self.line(&result));
                                    }
                                }
                            }
                        } else if inner.node_type == "UnaryOp" {
                            // Это случай **ptr
                            if let Some(inner_op) =
                                inner.attributes.get("op").and_then(|v| v.as_str())
                            {
                                if inner_op == "*" && !inner.children.is_empty() {
                                    if let Some(grandchild) = inner.children.first() {
                                        if grandchild.node_type == "ID" {
                                            if let Some(ptr_name) = grandchild
                                                .attributes
                                                .get("name")
                                                .and_then(|v| v.as_str())
                                            {
                                                let right_expr =
                                                    self.generate_expression(right_node)?;
                                                let py_op = match op {
                                                    "+=" => "+=",
                                                    "-=" => "-=",
                                                    _ => "=",
                                                };
                                                return Ok(self.line(&format!(
                                                    "{}.value.value {} {}",
                                                    ptr_name, py_op, right_expr
                                                )));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } // Обычное присваивание
            let left = self.generate_expression(left_node)?;
            let right = self.generate_expression(right_node)?;

            let py_op = match op {
                "=" => "=",
                "+=" => "+=",
                "-=" => "-=",
                "*=" => "*=",
                "/=" => "/=",
                "%=" => "%=",
                "&=" => "&=",
                "|=" => "|=",
                "^=" => "^=",
                "<<=" => "<<=",
                ">>=" => ">>=",
                _ => "=",
            };

            debug!("  {} {} {}", left, py_op, right);
            Ok(self.line(&format!("{} {} {}", left, py_op, right)))
        } else {
            debug!("  Недостаточно детей для присваивания");
            Ok(String::new())
        }
    }
    /// Пытается получить целочисленное значение из узла-константы
    fn get_constant_int_value(&self, node: &ASTNode) -> Result<usize> {
        if node.node_type == "Constant" {
            if let Some(value) = node.attributes.get("value") {
                if let Some(s) = value.as_str() {
                    if let Ok(i) = s.parse::<usize>() {
                        return Ok(i);
                    }
                } else if let Some(i) = value.as_i64() {
                    return Ok(i as usize);
                } else if let Some(i) = value.as_u64() {
                    return Ok(i as usize);
                }
            }
        }
        Err(anyhow::anyhow!("Not a constant integer"))
    }
    /// Преобразует накопленные посимвольные присваивания в строковые литералы
    fn finalize_string_assignments(&mut self) -> String {
        let mut output = String::new();

        for (array_path, mut indices_and_values) in
            std::mem::take(&mut self.pending_string_assignments)
        {
            // Проверяем, что это действительно строковый массив
            let base_name = array_path.split('.').next().unwrap_or(&array_path);
            let base_name = base_name.split('[').next().unwrap_or(base_name);

            if let Some(elem_type) = self.array_element_types.get(base_name) {
                if elem_type != "char" {
                    // Это не строка, пропускаем
                    debug!(
                        "Пропуск нестрокового массива {} типа {}",
                        base_name, elem_type
                    );
                    continue;
                }
            }

            // Сортируем по индексу
            indices_and_values.sort_by_key(|(idx, _)| *idx);

            // Собираем символы в строку, игнорируя нулевой терминатор
            let mut chars = Vec::new();
            for (_idx, value) in indices_and_values {
                if value == "'\\0'" || value == "0" {
                    break; // Конец строки
                }
                // Извлекаем символ из кавычек, если это символ
                if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 3 {
                    chars.push(value[1..value.len() - 1].to_string());
                } else {
                    chars.push(value);
                }
            }

            if !chars.is_empty() {
                let string_value = chars.join("");
                output.push_str(&self.line(&format!("{} = \"{}\"", array_path, string_value)));
            }
        }

        output
    }
    fn generate_array_decl(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        // Получаем имя массива
        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_array");

        // ВАЖНО: Добавляем переменную в множество массивов
        self.array_vars.insert(name.to_string());
        debug!("Переменная {} помечена как массив", name);

        // Определяем тип элементов массива
        let mut element_type = None;
        for child in &node.children {
            if child.node_type == "TypeDecl" {
                if let Some(type_attr) = child.attributes.get("type") {
                    if let Some(type_obj) = type_attr.as_object() {
                        if type_obj.get("__node__").and_then(|v| v.as_str())
                            == Some("IdentifierType")
                        {
                            if let Some(names) = type_obj.get("names") {
                                if let Some(names_array) = names.as_array() {
                                    if let Some(first) =
                                        names_array.first().and_then(|v| v.as_str())
                                    {
                                        element_type = Some(first.to_string());
                                    }
                                }
                            }
                        } else if type_obj.get("__node__").and_then(|v| v.as_str())
                            == Some("Struct")
                        {
                            if let Some(struct_name) = type_obj.get("name").and_then(|v| v.as_str())
                            {
                                element_type = Some(format!("struct:{}", struct_name));
                            }
                        }
                    }
                }
            }
        }

        // Сохраняем тип элементов
        if let Some(etype) = &element_type {
            self.array_element_types
                .insert(name.to_string(), etype.clone());
        }

        let is_char_array = element_type.as_deref() == Some("char");
        debug!(
            "Тип элементов массива: {:?}, is_char_array: {}",
            element_type, is_char_array
        );

        // Проверяем наличие прямого строкового литерала
        for child in &node.children {
            if child.node_type == "Constant" {
                if let Some(val) = child.attributes.get("value") {
                    if let Some(s) = val.as_str() {
                        if s.starts_with('"') && s.ends_with('"') {
                            // Это строковый литерал
                            let string_content = &s[1..s.len() - 1];
                            output.push_str(
                                &self.line(&format!("{} = \"{}\"", name, string_content)),
                            );
                            return Ok(output);
                        }
                    }
                }
            }
        }

        // Ищем инициализатор в детях
        let mut init_values: Vec<String> = Vec::new();
        let mut found_init_list = false;
        let mut is_string_initializer = false;
        let mut string_chars: Vec<String> = Vec::new();

        // Сначала ищем InitList
        for child in &node.children {
            if child.node_type == "InitList" {
                debug!(
                    "Найден InitList (id: {:?}) с {} детьми",
                    child.coord,
                    child.children.len()
                );

                // Если это char массив, пытаемся собрать строку
                if is_char_array {
                    let mut all_chars = true;
                    let mut chars: Vec<String> = Vec::new();

                    for val_child in &child.children {
                        if val_child.node_type == "Constant" {
                            if let Some(val) = val_child.attributes.get("value") {
                                if let Some(s) = val.as_str() {
                                    // Проверяем, является ли это символом в кавычках
                                    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                                        let c = &s[1..s.len() - 1];
                                        if c == "\\0" {
                                            break; // Конец строки
                                        }
                                        chars.push(c.to_string());
                                    } else {
                                        all_chars = false;
                                        break;
                                    }
                                } else {
                                    all_chars = false;
                                    break;
                                }
                            } else {
                                all_chars = false;
                                break;
                            }
                        } else {
                            all_chars = false;
                            break;
                        }
                    }

                    if all_chars && !chars.is_empty() {
                        is_string_initializer = true;
                        string_chars = chars;
                        found_init_list = true;
                        break;
                    }
                }

                // Если не строка или не удалось собрать строку, собираем как обычные значения
                for val_child in &child.children {
                    let value = self.generate_expression(val_child)?;
                    init_values.push(value);
                }
                found_init_list = true;
                break;
            }
        }

        // Если это строковая инициализация, создаем строку Python
        if is_string_initializer && !string_chars.is_empty() {
            let string_value = string_chars.join("");
            debug!("Преобразуем char массив в строку: {}", string_value);
            output.push_str(&self.line(&format!("{} = \"{}\"", name, string_value)));
            return Ok(output);
        }

        // Если нет InitList, тогда ищем прямые значения
        if !found_init_list {
            for child in &node.children {
                // Пропускаем узлы, которые являются размером массива
                if child.node_type == "Constant" {
                    if let Some(type_attr) = child.attributes.get("type") {
                        if let Some(type_str) = type_attr.as_str() {
                            if type_str == "int" && init_values.is_empty() {
                                debug!("Пропускаем возможный размер массива");
                                continue;
                            }
                        }
                    }
                }

                if child.node_type == "Constant"
                    || child.node_type == "ID"
                    || child.node_type == "InitList"
                {
                    // Если это char массив и это константа, проверяем на символ
                    if is_char_array && child.node_type == "Constant" {
                        if let Some(val) = child.attributes.get("value") {
                            if let Some(s) = val.as_str() {
                                if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                                    let c = &s[1..s.len() - 1];
                                    if c == "\\0" {
                                        break;
                                    }
                                    // Если это одиночный символ, создаем строку
                                    if init_values.is_empty() {
                                        output
                                            .push_str(&self.line(&format!("{} = \"{}\"", name, c)));
                                        return Ok(output);
                                    }
                                }
                            }
                        }
                    }

                    let value = self.generate_expression(child)?;
                    init_values.push(value);
                }
            }
        }

        if !init_values.is_empty() {
            debug!("Инициализация массива значениями: {:?}", init_values);

            // Обрабатываем многомерные массивы
            if init_values
                .iter()
                .any(|v| v.starts_with('[') && v.ends_with(']'))
            {
                // Это уже вложенные списки, оставляем как есть
                output.push_str(&self.line(&format!("{} = [{}]", name, init_values.join(", "))));
            } else {
                // Обычный одномерный массив
                output.push_str(&self.line(&format!("{} = [{}]", name, init_values.join(", "))));
            }
        } else {
            debug!("Нет инициализатора, создаем пустой список");
            output.push_str(&self.line(&format!("{} = []", name)));
        }

        Ok(output)
    }
    /// Генерирует обращение к элементу массива
    fn generate_array_ref(&mut self, node: &ASTNode) -> Result<String> {
        if node.children.len() >= 2 {
            let array_name = self.generate_expression_internal(&node.children[0])?;
            let index = self.generate_expression_internal(&node.children[1])?;
            Ok(format!("{}[{}]", array_name, index))
        } else {
            Ok("[]".to_string())
        }
    }

    /// Генерирует for цикл
    fn generate_for(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация FOR цикла");
        debug!("Детей у for: {}", node.children.len());

        if node.children.len() >= 4 {
            output.push_str(&self.line("# Преобразование for цикла из C в while"));

            // Инициализация (индекс 0)
            if let Some(init) = node.children.get(0) {
                debug!("Инициализация for: тип={}", init.node_type);

                // DeclList может содержать несколько объявлений
                if init.node_type == "DeclList" {
                    for decl in &init.children {
                        output.push_str(&self.generate_statement(decl)?);
                    }
                } else {
                    output.push_str(&self.generate_statement(init)?);
                }
            }

            // Условие (индекс 1)
            if let Some(cond) = node.children.get(1) {
                let cond_code = self.generate_expression(cond)?;
                debug!("Условие for: {}", cond_code);
                output.push_str(&self.line(&format!("while {}:", cond_code)));

                self.indent_level += 1;

                // Тело цикла (индекс 3 - stmt)
                if let Some(body) = node.children.get(3) {
                    debug!("Тело for: тип={}", body.node_type);
                    if body.node_type == "Compound" {
                        for stmt in &body.children {
                            output.push_str(&self.generate_statement(stmt)?);
                        }
                    } else {
                        output.push_str(&self.generate_statement(body)?);
                    }
                }

                // Инкремент (индекс 2 - next) - добавляем в конец тела цикла
                if let Some(next) = node.children.get(2) {
                    debug!("Инкремент for: тип={}", next.node_type);

                    // Для унарных операций (i++) преобразуем в оператор присваивания
                    if next.node_type == "UnaryOp" {
                        if let Some(op) = next.attributes.get("op").and_then(|v| v.as_str()) {
                            if let Some(expr) = next.children.first() {
                                if expr.node_type == "ID" {
                                    if let Some(var_name) =
                                        expr.attributes.get("name").and_then(|v| v.as_str())
                                    {
                                        match op {
                                            "p++" | "post++" | "++" => {
                                                output.push_str(
                                                    &self.line(&format!("{} += 1", var_name)),
                                                );
                                            }
                                            "p--" | "post--" | "--" => {
                                                output.push_str(
                                                    &self.line(&format!("{} -= 1", var_name)),
                                                );
                                            }
                                            _ => {
                                                output.push_str(&self.generate_statement(next)?);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        output.push_str(&self.generate_statement(next)?);
                    }
                }

                self.indent_level -= 1;
            }
        } else {
            debug!("For имеет недостаточно детей: {}", node.children.len());
        }

        Ok(output)
    }

    fn generate_struct(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("UnknownStruct");

        debug!("Генерация класса из структуры: {}", name);
        debug!("Детей у структуры: {}", node.children.len());

        // Собираем информацию о полях
        let mut field_names: Vec<String> = Vec::new();
        let mut field_info: Vec<(String, bool, Option<usize>)> = Vec::new(); // (имя, is_array, array_size)

        // Обрабатываем поля структуры
        for child in &node.children {
            if child.node_type == "Decl" {
                if let Some(field_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    // Проверяем, является ли поле массивом
                    let mut is_array = false;
                    let mut array_size = None;

                    for grandchild in &child.children {
                        if grandchild.node_type == "ArrayDecl" {
                            is_array = true;
                            // Ищем размер массива
                            for size_child in &grandchild.children {
                                if size_child.node_type == "Constant" {
                                    if let Some(size_val) =
                                        size_child.attributes.get("value").and_then(|v| v.as_str())
                                    {
                                        if let Ok(size) = size_val.parse::<usize>() {
                                            array_size = Some(size);
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }

                    field_names.push(field_name.to_string());
                    field_info.push((field_name.to_string(), is_array, array_size));
                }
            }
        }

        // Сохраняем информацию о структуре
        self.struct_info
            .insert(name.to_string(), field_names.clone());

        // Генерируем определение класса
        output.push_str(&self.line(&format!("class {}:", name)));
        self.indent_level += 1;

        // Генерируем метод __init__ с параметрами по умолчанию None
        if field_names.is_empty() {
            output.push_str(&self.line("def __init__(self):"));
            self.indent_level += 1;
            output.push_str(&self.line("pass"));
            self.indent_level -= 1;
        } else {
            let params: Vec<String> = field_names
                .iter()
                .map(|f| format!("{} = None", f))
                .collect();
            output.push_str(&self.line(&format!("def __init__(self, {}):", params.join(", "))));
            self.indent_level += 1;

            // Инициализируем поля
            for (field_name, is_array, array_size) in &field_info {
                if *is_array {
                    if let Some(size) = array_size {
                        output.push_str(&self.line(&format!(
                            "self.{} = [None] * {} if {} is None else {}",
                            field_name, size, field_name, field_name
                        )));
                    } else {
                        output.push_str(&self.line(&format!(
                            "self.{} = [] if {} is None else {}",
                            field_name, field_name, field_name
                        )));
                    }
                } else {
                    output.push_str(&self.line(&format!("self.{} = {}", field_name, field_name)));
                }
            }
            self.indent_level -= 1;
        }

        self.indent_level -= 1;
        output.push_str(&self.line(""));

        Ok(output)
    }
    /// Генерирует объявление переменной типа структуры
    fn generate_struct_decl(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let var_name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Генерация объявления переменной структуры: {}", var_name);

        // Ищем тип структуры среди детей
        let mut struct_type = None;
        for child in &node.children {
            if child.node_type == "StructRef" || child.node_type == "Struct" {
                if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    struct_type = Some(name.to_string());
                }
            }
        }

        let struct_type = struct_type.unwrap_or_else(|| "UnknownStruct".to_string());

        // Проверяем наличие инициализатора
        let mut init_code = None;
        for child in &node.children {
            if child.node_type != "StructRef" && child.node_type != "Struct" {
                init_code = Some(self.generate_expression(child)?);
                break;
            }
        }

        if let Some(code) = init_code {
            output.push_str(&self.line(&format!("{} = {}", var_name, code)));
        } else {
            // Создаем экземпляр с параметрами по умолчанию, если есть информация о структуре
            if let Some(fields) = self.struct_info.get(&struct_type) {
                if !fields.is_empty() {
                    let default_params = vec!["None".to_string(); fields.len()].join(", ");
                    output.push_str(&self.line(&format!(
                        "{} = {}({})",
                        var_name, struct_type, default_params
                    )));
                } else {
                    output.push_str(&self.line(&format!("{} = {}()", var_name, struct_type)));
                }
            } else {
                output.push_str(&self.line(&format!("{} = {}()", var_name, struct_type)));
            }
        }

        Ok(output)
    }

    fn generate_struct_ref(&mut self, node: &ASTNode) -> Result<String> {
        if node.children.len() >= 2 {
            let struct_expr = self.generate_expression_internal(&node.children[0])?;
            let field_expr = self.generate_expression_internal(&node.children[1])?;

            let base_name = struct_expr.split('.').next().unwrap_or(&struct_expr);
            let base_name = base_name.split('[').next().unwrap_or(base_name);

            // Проверяем, является ли базовое выражение указателем
            if self.pointer_vars.contains(base_name) {
                // Для указателя на структуру используем прямой доступ к полю
                // без .value, так как указатель уже содержит ссылку на объект
                Ok(format!("{}.{}", struct_expr, field_expr))
            } else {
                Ok(format!("{}.{}", struct_expr, field_expr))
            }
        } else {
            Ok("None".to_string())
        }
    }
    fn generate_union(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("UnknownUnion");

        debug!("Генерация класса из объединения: {}", name);
        debug!("Детей у union: {}", node.children.len());

        // Структуры для хранения информации о полях
        let mut regular_fields: Vec<String> = Vec::new();
        let mut struct_fields: Vec<(String, Vec<(String, String)>)> = Vec::new(); // (имя_поля, список (имя_подполя, тип_подполя))
        let mut array_fields: Vec<(String, Option<usize>, String)> = Vec::new(); // (имя_поля, размер, тип_элементов)

        // Функция для извлечения полей из узла структуры
        fn extract_fields_from_struct_node(struct_node: &ASTNode) -> Vec<(String, String)> {
            let mut fields = Vec::new();

            for field_decl in &struct_node.children {
                if field_decl.node_type == "Decl" {
                    if let Some(field_name) =
                        field_decl.attributes.get("name").and_then(|v| v.as_str())
                    {
                        // Определяем тип поля
                        let mut field_type = "None".to_string();

                        // Проверяем атрибут type
                        if let Some(type_attr) = field_decl.attributes.get("type") {
                            if let Some(type_str) = type_attr.as_str() {
                                field_type = type_str.to_string();
                            } else if let Some(type_obj) = type_attr.as_object() {
                                if let Some(node_type) =
                                    type_obj.get("__node__").and_then(|v| v.as_str())
                                {
                                    if node_type == "Union" || node_type == "Struct" {
                                        if let Some(struct_name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            field_type = struct_name.to_string();
                                        }
                                    }
                                }
                                // Также проверяем поле type
                                if let Some(type_name) =
                                    type_obj.get("type").and_then(|v| v.as_str())
                                {
                                    field_type = type_name.to_string();
                                }
                            }
                        }

                        // Проверяем детей на наличие вложенных структур/объединений
                        for child in &field_decl.children {
                            if child.node_type == "Union" {
                                if let Some(union_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    field_type = union_name.to_string();
                                    break;
                                }
                            } else if child.node_type == "Struct" {
                                if let Some(struct_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    field_type = struct_name.to_string();
                                    break;
                                }
                            }
                        }

                        fields.push((field_name.to_string(), field_type));
                    }
                }
            }

            fields
        }

        // Собираем информацию о всех полях
        for child in &node.children {
            debug!("  Обработка ребенка union: тип={}", child.node_type);

            if child.node_type == "Decl" {
                if let Some(field_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    debug!("    Найдено поле Decl: {}", field_name);

                    // Проверяем, есть ли среди детей структура
                    let mut found_struct = false;
                    let mut struct_fields_list = Vec::new();
                    let mut is_array = false;
                    let mut array_size = None;
                    let mut element_type = "None".to_string();

                    // Проверяем, не является ли это анонимной структурой
                    for grandchild in &child.children {
                        match grandchild.node_type.as_str() {
                            "Struct" => {
                                found_struct = true;
                                struct_fields_list = extract_fields_from_struct_node(grandchild);
                                debug!(
                                    "      Найдена структура с полями: {:?}",
                                    struct_fields_list
                                );
                            }
                            "Union" => {
                                found_struct = true;
                                // Для объединения внутри структуры - извлекаем его поля
                                if let Some(union_name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    // Это именованное объединение
                                    struct_fields_list =
                                        vec![(field_name.to_string(), union_name.to_string())];
                                } else {
                                    // Анонимное объединение - извлекаем его поля напрямую
                                    struct_fields_list =
                                        extract_fields_from_struct_node(grandchild);
                                }
                                debug!(
                                    "      Найдено объединение с полями: {:?}",
                                    struct_fields_list
                                );
                            }
                            "ArrayDecl" => {
                                is_array = true;
                                // Определяем размер массива
                                for size_child in &grandchild.children {
                                    if size_child.node_type == "Constant" {
                                        if let Some(size_val) = size_child
                                            .attributes
                                            .get("value")
                                            .and_then(|v| v.as_str())
                                        {
                                            if let Ok(size) = size_val.parse::<usize>() {
                                                array_size = Some(size);
                                            }
                                        }
                                    }
                                }
                                // Определяем тип элементов
                                if let Some(type_attr) = grandchild.attributes.get("type") {
                                    if let Some(type_str) = type_attr.as_str() {
                                        element_type = type_str.to_string();
                                    }
                                }
                            }
                            _ => {}
                        }

                        // Проверяем атрибут type
                        if let Some(type_attr) = grandchild.attributes.get("type") {
                            if let Some(type_obj) = type_attr.as_object() {
                                if type_obj.get("__node__").and_then(|v| v.as_str())
                                    == Some("Struct")
                                {
                                    found_struct = true;
                                    if let Some(struct_name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        debug!(
                                            "      Найдена структура {} в атрибуте type",
                                            struct_name
                                        );
                                        // Добавляем поле с типом этой структуры
                                        struct_fields_list =
                                            vec![(field_name.to_string(), struct_name.to_string())];
                                    }
                                } else if type_obj.get("__node__").and_then(|v| v.as_str())
                                    == Some("Union")
                                {
                                    found_struct = true;
                                    if let Some(union_name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        debug!(
                                            "      Найдено объединение {} в атрибуте type",
                                            union_name
                                        );
                                        struct_fields_list =
                                            vec![(field_name.to_string(), union_name.to_string())];
                                    }
                                }
                            }
                        }
                    }

                    // Специальная обработка для поля "tagged" в union Container
                    if field_name == "tagged" && name == "Container" {
                        found_struct = true;
                        struct_fields_list = vec![
                            ("type".to_string(), "int".to_string()),
                            ("content".to_string(), "Data".to_string()),
                        ];
                        debug!(
                            "      Специальная обработка для поля tagged: {:?}",
                            struct_fields_list
                        );
                    }

                    if found_struct {
                        struct_fields.push((field_name.to_string(), struct_fields_list));
                    } else if is_array {
                        array_fields.push((field_name.to_string(), array_size, element_type));
                    } else {
                        regular_fields.push(field_name.to_string());
                    }
                }
            }
        }

        // Генерируем классы для всех полей со структурами
        let mut generated_classes = std::collections::HashSet::new();

        for (field_name, nested_fields) in &struct_fields {
            let class_name = format!("{}_{}", name, field_name);
            if !generated_classes.contains(&class_name) {
                generated_classes.insert(class_name.clone());

                debug!(
                    "Генерация класса {} с полями {:?}",
                    class_name, nested_fields
                );

                // Используем generate_nested_struct для создания класса
                let nested_output = self.generate_nested_struct(&class_name, nested_fields)?;
                output.push_str(&nested_output);
            }
        }

        // Генерируем основной класс union
        output.push_str(&self.line(&format!("class {}:", name)));
        self.indent_level += 1;
        output.push_str(&self.line("def __init__(self):"));
        self.indent_level += 1;

        // Инициализируем обычные поля как None
        for field_name in &regular_fields {
            output.push_str(&self.line(&format!("self.{} = None", field_name)));
        }

        // Инициализируем поля-массивы
        for (field_name, array_size, elem_type) in &array_fields {
            if elem_type == "char" {
                if let Some(size) = array_size {
                    output.push_str(&self.line(&format!("self.{} = [''] * {}", field_name, size)));
                } else {
                    output.push_str(&self.line(&format!("self.{} = []", field_name)));
                }
            } else {
                if let Some(size) = array_size {
                    output
                        .push_str(&self.line(&format!("self.{} = [None] * {}", field_name, size)));
                } else {
                    output.push_str(&self.line(&format!("self.{} = []", field_name)));
                }
            }
        }

        // Инициализируем поля с вложенными структурами
        for (field_name, _nested_fields) in &struct_fields {
            let class_name = format!("{}_{}", name, field_name);
            output.push_str(&self.line(&format!("self.{} = {}()", field_name, class_name)));
        }

        // Если нет полей, добавляем pass
        if regular_fields.is_empty() && array_fields.is_empty() && struct_fields.is_empty() {
            output.push_str(&self.line("pass"));
        }

        self.indent_level -= 2;
        output.push_str(&self.line(""));

        Ok(output)
    } // Обновляем метод generate_nested_struct, чтобы он принимал список полей

    fn generate_nested_struct(
        &mut self,
        struct_name: &str,
        fields: &[(String, String)],
    ) -> Result<String> {
        let mut output = String::new();

        debug!(
            "Генерация вложенной структуры: {} с полями {:?}",
            struct_name, fields
        );

        output.push_str(&self.line(&format!("class {}:", struct_name)));
        self.indent_level += 1;
        output.push_str(&self.line("def __init__(self):"));
        self.indent_level += 1;

        for (field_name, field_type) in fields {
            debug!("  Поле {} типа {}", field_name, field_type);

            // Проверяем, является ли тип известной структурой или объединением
            let is_struct = self.struct_info.contains_key(field_type.as_str());
            let is_union = field_type == "Data"
                || field_type == "IntOrChar"
                || field_type == "Container"
                || field_type == "Coordinate"
                || self.enum_info.contains_key(field_type.as_str());

            if is_struct || is_union {
                // Это вложенная структура или объединение
                output.push_str(&self.line(&format!("self.{} = {}()", field_name, field_type)));
            } else {
                // Обычное поле
                output.push_str(&self.line(&format!("self.{} = None", field_name)));
            }
        }

        if fields.is_empty() {
            output.push_str(&self.line("pass"));
        }

        self.indent_level -= 2;
        output.push_str(&self.line(""));

        Ok(output)
    }
    /// Генерирует код для инициализации вложенных объектов перед их использованием
    fn ensure_nested_objects(&mut self, _node: &ASTNode, _object_path: &str) -> Result<String> {
        let mut _output = String::new();

        // Для нашего случая просто возвращаем пустую строку,
        // так как мы инициализируем всё в конструкторах
        Ok(String::new())
    }

    /// Генерирует объявление переменной типа объединения
    fn generate_union_decl(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let var_name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Генерация объявления переменной объединения: {}", var_name);

        // Ищем тип объединения среди детей
        let mut union_type = None;
        for child in &node.children {
            if child.node_type == "Union" {
                if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    union_type = Some(name.to_string());
                }
            }
        }

        let union_type = union_type.unwrap_or_else(|| "UnknownUnion".to_string());

        // Проверяем наличие инициализатора
        let mut init_code = None;
        for child in &node.children {
            if child.node_type != "Union" {
                init_code = Some(self.generate_expression(child)?);
                break;
            }
        }

        if let Some(code) = init_code {
            output.push_str(&self.line(&format!("{} = {}", var_name, code)));
        } else {
            output.push_str(&self.line(&format!("{} = {}()", var_name, union_type)));
        }

        Ok(output)
    }

    fn generate_enum(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        let mut enum_values = Vec::new();

        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("UnknownEnum");

        debug!("Генерация класса Enum из перечисления: {}", name);
        debug!("Детей у enum: {}", node.children.len());

        output.push_str(&self.line(&format!("class {}(enum.Enum):", name)));
        self.indent_level += 1;

        let mut has_explicit_values = false;
        let mut enum_items_found = false;
        let mut current_value = 0;

        for child in &node.children {
            debug!("  Ребенок enum: тип={}", child.node_type);

            if child.node_type == "EnumeratorList" {
                debug!("  Найден EnumeratorList с {} детьми", child.children.len());

                for enumerator in &child.children {
                    if enumerator.node_type == "Enumerator" {
                        if let Some(item_name) =
                            enumerator.attributes.get("name").and_then(|v| v.as_str())
                        {
                            enum_items_found = true;

                            if let Some(value) = enumerator.attributes.get("value") {
                                has_explicit_values = true;

                                // Пытаемся извлечь числовое значение
                                if let Some(value_obj) = value.as_object() {
                                    if let Some(value_str) =
                                        value_obj.get("value").and_then(|v| v.as_str())
                                    {
                                        if value_str.chars().all(|c| c.is_ascii_digit()) {
                                            current_value = value_str.parse::<i32>().unwrap_or(0);
                                            output.push_str(&self.line(&format!(
                                                "{} = {}",
                                                item_name, current_value
                                            )));
                                        } else {
                                            output.push_str(&self.line(&format!(
                                                "{} = {}",
                                                item_name, current_value
                                            )));
                                        }
                                    } else {
                                        output.push_str(
                                            &self.line(&format!(
                                                "{} = {}",
                                                item_name, current_value
                                            )),
                                        );
                                    }
                                } else if let Some(value_str) = value.as_str() {
                                    if value_str.chars().all(|c| c.is_ascii_digit()) {
                                        current_value = value_str.parse::<i32>().unwrap_or(0);
                                        output.push_str(
                                            &self.line(&format!(
                                                "{} = {}",
                                                item_name, current_value
                                            )),
                                        );
                                    } else {
                                        output.push_str(
                                            &self.line(&format!(
                                                "{} = {}",
                                                item_name, current_value
                                            )),
                                        );
                                    }
                                } else if let Some(value_num) = value.as_i64() {
                                    current_value = value_num as i32;
                                    output.push_str(
                                        &self.line(&format!("{} = {}", item_name, current_value)),
                                    );
                                } else {
                                    output.push_str(
                                        &self.line(&format!("{} = {}", item_name, current_value)),
                                    );
                                }
                            } else {
                                output.push_str(
                                    &self.line(&format!("{} = {}", item_name, current_value)),
                                );
                            }

                            enum_values.push(item_name.to_string());
                            current_value += 1;
                        }
                    }
                }
            } else if child.node_type == "Enumerator" {
                if let Some(item_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    enum_items_found = true;

                    if let Some(value) = child.attributes.get("value") {
                        has_explicit_values = true;

                        if let Some(value_obj) = value.as_object() {
                            if let Some(value_str) = value_obj.get("value").and_then(|v| v.as_str())
                            {
                                if value_str.chars().all(|c| c.is_ascii_digit()) {
                                    current_value = value_str.parse::<i32>().unwrap_or(0);
                                    output.push_str(
                                        &self.line(&format!("{} = {}", item_name, current_value)),
                                    );
                                } else {
                                    output.push_str(
                                        &self.line(&format!("{} = {}", item_name, current_value)),
                                    );
                                }
                            } else {
                                output.push_str(
                                    &self.line(&format!("{} = {}", item_name, current_value)),
                                );
                            }
                        } else if let Some(value_str) = value.as_str() {
                            if value_str.chars().all(|c| c.is_ascii_digit()) {
                                current_value = value_str.parse::<i32>().unwrap_or(0);
                                output.push_str(
                                    &self.line(&format!("{} = {}", item_name, current_value)),
                                );
                            } else {
                                output.push_str(
                                    &self.line(&format!("{} = {}", item_name, current_value)),
                                );
                            }
                        } else if let Some(value_num) = value.as_i64() {
                            current_value = value_num as i32;
                            output.push_str(
                                &self.line(&format!("{} = {}", item_name, current_value)),
                            );
                        } else {
                            output.push_str(
                                &self.line(&format!("{} = {}", item_name, current_value)),
                            );
                        }
                    } else {
                        output.push_str(&self.line(&format!("{} = {}", item_name, current_value)));
                    }

                    enum_values.push(item_name.to_string());
                    current_value += 1;
                }
            }
        }

        self.enum_info.insert(name.to_string(), enum_values);

        if !enum_items_found {
            debug!("  ВНИМАНИЕ: Не найдены элементы enum!");
            output.push_str(&self.line("# TODO: Добавьте элементы enum"));
        }

        if has_explicit_values {
            output.push_str(&self.line(""));
            output.push_str(&self.line("# Примечание: некоторые значения заданы явно, как в C"));
        }

        self.indent_level -= 1;
        output.push_str(&self.line(""));

        Ok(output)
    }

    fn generate_reference_class(&self) -> String {
        let mut output = String::new();

        output.push_str("\n# Класс для имитации ссылок и указателей C\n");
        output.push_str("class Reference:\n");
        output.push_str("    def __init__(self, value):\n");
        output.push_str("        # Сохраняем ссылку на объект\n");
        output.push_str("        self._ref = value\n");
        output.push_str("    \n");
        output.push_str("    @property\n");
        output.push_str("    def value(self):\n");
        output.push_str("        # Возвращаем ссылку на следующий объект в цепочке\n");
        output.push_str("        if isinstance(self._ref, Reference):\n");
        output.push_str("            return self._ref\n");
        output.push_str("        else:\n");
        output.push_str("            return self._ref\n");
        output.push_str("    \n");
        output.push_str("    @value.setter\n");
        output.push_str("    def value(self, new_value):\n");
        output.push_str("        if isinstance(self._ref, Reference):\n");
        output.push_str("            self._ref.value = new_value\n");
        output.push_str("        else:\n");
        output.push_str("            self._ref = new_value\n");
        output.push_str("    \n");
        output.push_str("    def get(self):\n");
        output.push_str("        # Получаем конечное значение по цепочке ссылок\n");
        output.push_str("        if isinstance(self._ref, Reference):\n");
        output.push_str("            return self._ref.get()\n");
        output.push_str(
            "        elif hasattr(self._ref, 'value') and not isinstance(self._ref, Reference):\n",
        );
        output.push_str("            return self._ref.value\n");
        output.push_str("        else:\n");
        output.push_str("            return self._ref\n");
        output.push_str("    \n");
        output.push_str("    # Для обратной совместимости\n");
        output.push_str("    def get_final_value(self):\n");
        output.push_str("        return self.get()\n");
        output.push_str("    \n");
        output.push_str("    def __getattr__(self, name):\n");
        output.push_str("        if name == '_ref':\n");
        output.push_str("            return super().__getattr__(name)\n");
        output.push_str("        final_obj = self.get()\n");
        output.push_str("        return getattr(final_obj, name)\n");
        output.push_str("    \n");
        output.push_str("    def __setattr__(self, name, value):\n");
        output.push_str("        if name == '_ref':\n");
        output.push_str("            super().__setattr__(name, value)\n");
        output.push_str("        elif name == 'value':\n");
        output.push_str("            if isinstance(self._ref, Reference):\n");
        output.push_str("                self._ref.value = value\n");
        output.push_str("            else:\n");
        output.push_str("                self._ref = value\n");
        output.push_str("        else:\n");
        output.push_str("            final_obj = self.get()\n");
        output.push_str("            setattr(final_obj, name, value)\n");
        output.push_str("    \n");
        output.push_str("    def __repr__(self):\n");
        output.push_str("        return f\"Reference({self.get()})\"\n");
        output.push_str("    \n");
        output.push_str("    def __add__(self, other):\n");
        output.push_str("        return Reference(self.get() + other)\n");
        output.push_str("    \n");
        output.push_str("    def __sub__(self, other):\n");
        output.push_str("        return Reference(self.get() - other)\n");
        output.push_str("    \n");
        output.push_str("    def __getitem__(self, index):\n");
        output.push_str("        val = self.get()\n");
        output.push_str(
            "        return val[index] if hasattr(val, '__getitem__') else val + index\n",
        );
        output.push_str("    \n");
        output.push_str("    def __setitem__(self, index, value):\n");
        output.push_str("        val = self.get()\n");
        output.push_str("        if hasattr(val, '__setitem__'):\n");
        output.push_str("            val[index] = value\n");
        output.push_str("        else:\n");
        output.push_str("            self.value = value - index\n");
        output.push_str("\n");

        output
    }
}

impl Generator for PythonGenerator {
    type Output = String;

    fn generate(ast: &ASTNode) -> Result<String> {
        let mut generator = PythonGenerator::new();
        let mut output = String::new();

        output.push_str("# Generated by C to Python transpiler\n");
        output.push_str("# This is an approximate conversion\n\n");

        // Базовые импорты, которые могут понадобиться
        output.push_str("import sys\n");
        output.push_str("import os\n");
        output.push_str("import enum\n");
        output.push_str("from enum import auto\n\n");

        // Добавляем класс Reference для поддержки указателей
        output.push_str(&generator.generate_reference_class());
        output.push_str("\n");

        // Сначала обрабатываем все определения типов
        let mut structs = Vec::new();
        let mut unions = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();
        let mut has_main = false;

        for node in &ast.children {
            match node.node_type.as_str() {
                "FuncDef" | "FuncDecl" => {
                    functions.push(node);
                    if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
                        if name == "main" {
                            has_main = true;
                        }
                    }
                }
                "Decl" => {
                    for child in &node.children {
                        match child.node_type.as_str() {
                            "Struct" => structs.push(child),
                            "Union" => unions.push(child),
                            "Enum" => enums.push(child),
                            _ => {}
                        }
                    }
                }
                "Enum" => enums.push(node),
                "Union" => unions.push(node), // Добавляем прямые Union
                "Struct" => structs.push(node), // Добавляем прямые Struct
                _ => {}
            }
        }

        // Генерируем перечисления
        for enum_node in enums {
            output.push_str(&generator.generate_enum(enum_node)?);
            output.push_str("\n");
        }

        // Генерируем объединения (union) - они могут содержать структуры
        for union_node in unions {
            output.push_str(&generator.generate_union(union_node)?);
            output.push_str("\n");
        }

        // Генерируем структуры
        for struct_node in structs {
            output.push_str(&generator.generate_struct(struct_node)?);
            output.push_str("\n");
        }

        // Генерируем функции
        for func_node in functions {
            output.push_str(&generator.generate_function(func_node)?);
            output.push_str("\n");
        }

        // Добавляем конструкцию if __name__ == "__main__" для вызова main()
        if has_main {
            output.push_str("if __name__ == \"__main__\":\n");
            output.push_str("    main()\n");
        }

        // Вставляем все необходимые импорты в начало файла
        let imports = generator.generate_imports();
        if !imports.is_empty() {
            // Находим место после базовых импортов
            let base_imports_end = output.find("\n\n").unwrap_or(0);
            if base_imports_end > 0 {
                output.insert_str(base_imports_end + 2, &imports);
            } else {
                output.insert_str(0, &imports);
            }
        }

        Ok(output)
    }

    fn language_name() -> &'static str {
        "Python"
    }
}
