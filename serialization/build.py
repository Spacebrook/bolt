import os
import sys
import ast
import importlib.util
from pathlib import Path

class ClassInfo:
    def __init__(self, file_path, class_name):
        self.file_path = file_path
        self.class_name = class_name

def main():
    classes_with_bolt_codegen = []

    py_project_dir = os.environ.get('PY_PROJECT_DIR')
    if not py_project_dir:
        raise EnvironmentError("PY_PROJECT_DIR environment variable is not set")

    for root, dirs, files in os.walk(py_project_dir):
        for filename in files:
            if filename.endswith('.py'):
                file_path = os.path.join(root, filename)
                with open(file_path, 'r', encoding='utf-8') as f:
                    code = f.read()

                try:
                    ast_program = ast.parse(code, filename=file_path)
                except Exception as e:
                    print(f"Failed to parse Python code in {file_path}: {e}", file=sys.stderr)
                    continue

                for node in ast_program.body:
                    if isinstance(node, ast.ClassDef):
                        class_name = node.name
                        has_bolt_codegen = False
                        for class_stmt in node.body:
                            if isinstance(class_stmt, ast.FunctionDef) and class_stmt.name == 'bolt_codegen':
                                has_bolt_codegen = True
                                break
                        if has_bolt_codegen:
                            classes_with_bolt_codegen.append(ClassInfo(file_path, class_name))

    for class_info in classes_with_bolt_codegen:
        file_path = class_info.file_path
        class_name = class_info.class_name

        module_name = Path(file_path).stem
        spec = importlib.util.spec_from_file_location(module_name, file_path)
        if spec is None:
            print(f"Failed to create module spec for {file_path}", file=sys.stderr)
            continue
        module = importlib.util.module_from_spec(spec)
        try:
            spec.loader.exec_module(module)
        except Exception as e:
            print(f"Failed to execute module {file_path}: {e}", file=sys.stderr)
            continue

        try:
            cls = getattr(module, class_name)
        except AttributeError:
            print(f"Failed to get class '{class_name}' from module '{module_name}'", file=sys.stderr)
            continue

        try:
            instance = cls()
        except Exception as e:
            print(f"Failed to instantiate class '{class_name}': {e}", file=sys.stderr)
            continue

        try:
            result = instance.bolt_codegen()
        except Exception as e:
            print(f"Failed to call 'bolt_codegen' on '{class_name}': {e}", file=sys.stderr)
            continue

        if not isinstance(result, dict):
            print(f"'bolt_codegen' did not return a dictionary in class '{class_name}'", file=sys.stderr)
            continue

        print(f"Result from '{class_name}': {result}")

if __name__ == '__main__':
    main()
