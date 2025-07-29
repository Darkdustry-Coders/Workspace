class JavaVersionCheck {
    public static void main(String[] args) {
        if (Runtime.version().feature() >= 17) {
            System.out.println(Runtime.version().feature());
        } else {
            System.exit(1);
        }
    }
}
