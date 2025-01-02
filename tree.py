import os
import shutil
from datetime import datetime

def create_directory_tree(start_path=os.getcwd(), output_folder='flat'):
    # Folders and patterns to ignore
    IGNORE_PATTERNS = {
        'node_modules',
        'build',
        'dist',
        '__pycache__',
        '.git',
        '.idea',
        '.vscode',
        'venv',
        'env',
        '.next'
    }
    
    def generate_ascii_art():
        art = [
            "     /\\     ",
            "    /  \\    ",
            "   /    \\   ",
            "  /______\\  ",
            "    ||||    ",
            "    ||||    ",
        ]
        print("\n".join(art))
        print("\nDirectory Tree Generator")
        print("=" * 20 + "\n")

    def should_ignore(path):
        return any(ignore in path.split(os.sep) for ignore in IGNORE_PATTERNS)
        
    # Now we can call generate_ascii_art after it's defined
    generate_ascii_art()
    
    # Generate visual tree BEFORE creating output folder
    print("\nGenerating visual directory structure...")
    print(f"\nDirectory Tree for: {os.path.abspath(start_path)}")
    
    def build_visual_tree(path, indent="", is_last=True):
        basename = os.path.basename(path)
        if not basename:  # Handle root directory
            basename = path
            
        # Skip anything that starts with the output_folder name
        if basename.startswith(output_folder):
            return ""
            
        # Skip if should be ignored
        if should_ignore(path):
            return ""
            
        visual = indent
        if indent:  # Not the root
            visual += "└── " if is_last else "├── "
            visual += basename + "\n"
        else:  # Root directory
            visual += basename + "\n"

        try:
            items = os.listdir(path)
            # Filter out anything that starts with flat and ignored items
            items = [item for item in items 
                    if not item.startswith(output_folder)
                    and not should_ignore(os.path.join(path, item))]
            items.sort()
            
            for i, item in enumerate(items):
                item_path = os.path.join(path, item)
                if os.path.isdir(item_path):
                    new_indent = indent + ("    " if is_last else "│   ")
                    visual += build_visual_tree(item_path, new_indent, i == len(items) - 1)
                else:
                    visual += indent
                    visual += ("└── " if i == len(items) - 1 else "├── ") + item + "\n"
                    
            return visual
        except PermissionError:
            return visual + indent + "  [Permission Denied]\n"
    
    # Generate and display the visual tree
    visual_tree = build_visual_tree(start_path)
    print("\nDirectory Structure:")
    print(visual_tree)
    
    # Now create the output folder with timestamp
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    output_folder = f"{output_folder}_{timestamp}"
    os.makedirs(output_folder, exist_ok=True)
    print(f"\nCreated output folder: {output_folder}")
    
    # Initialize the tree structure string
    tree_structure = []
    
    def generate_tree(path, prefix=""):
        if should_ignore(path):
            print(f"Skipping ignored directory: {path}")
            return
            
        if os.path.basename(path) == output_folder:
            return
            
        try:
            files = os.listdir(path)
        except PermissionError:
            print(f"Permission denied: {path}")
            return
            
        files.sort()
        
        for i, file in enumerate(files):
            current_path = os.path.join(path, file)
            
            if should_ignore(current_path):
                print(f"Skipping ignored file/directory: {current_path}")
                continue
                
            is_last = i == len(files) - 1
            connector = "└── " if is_last else "├── "
            tree_line = f"{prefix}{connector}{file}"
            print(tree_line)
            tree_structure.append(tree_line)
            
            if os.path.isdir(current_path):
                next_prefix = prefix + ("    " if is_last else "│   ")
                generate_tree(current_path, next_prefix)
            else:
                # Copy file to output folder with a flat structure
                try:
                    # Create a unique filename if duplicates exist
                    base_name = os.path.basename(current_path)
                    name, ext = os.path.splitext(base_name)
                    output_path = os.path.join(output_folder, base_name)
                    counter = 1
                    
                    # If file already exists, add a number to the filename
                    while os.path.exists(output_path):
                        output_path = os.path.join(output_folder, f"{name}_{counter}{ext}")
                        counter += 1
                    
                    shutil.copy2(current_path, output_path)
                    print(f"Copied: {base_name} -> {os.path.basename(output_path)}")
                except (shutil.SameFileError, PermissionError) as e:
                    print(f"Error copying {current_path}: {e}")

    # Generate the tree starting from current directory
    print(f"\nDirectory Tree for: {os.path.abspath(start_path)}")
    print(".")
    generate_tree(start_path)
    
    # Save both the list and visual representation
    tree_file = os.path.join(output_folder, "directory_tree.txt")
    with open(tree_file, 'w', encoding='utf-8') as f:
        f.write(f"Directory Tree for: {os.path.abspath(start_path)}\n\n")
        f.write("Visual Structure:\n")
        f.write(visual_tree)
        f.write("\n\nDetailed Structure:\n")
        f.write(".\n")
        f.write("\n".join(tree_structure))
    
    print("\nTree structure has been saved to:", tree_file)
    print("All files have been copied to:", output_folder)

if __name__ == "__main__":
    create_directory_tree()