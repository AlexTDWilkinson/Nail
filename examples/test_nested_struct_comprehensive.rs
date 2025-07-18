use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Vec2 {
        x: f64,
        y: f64,
    }
    #[derive(Debug, Clone)]
    struct Vec3 {
        x: f64,
        y: f64,
        z: f64,
    }
    #[derive(Debug, Clone)]
    struct Transform {
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
    }
    #[derive(Debug, Clone)]
    struct GameObject {
        name: String,
        transform: Transform,
        active: bool,
    }
    #[derive(Debug, PartialEq)]
    enum ColliderType {
        None,
        Box,
        Sphere,
        Mesh,
    }
    #[derive(Debug, Clone)]
    struct Collider {
        size: Vec3,
        offset: Vec3,
    }
    #[derive(Debug, Clone)]
    struct Scene {
        name: String,
    }
    println!("{}", "Comprehensive nested struct test completed!".to_string());
}
