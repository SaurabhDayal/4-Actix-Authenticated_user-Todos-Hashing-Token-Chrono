use actix_web::http::header::ContentType;
use actix_web::{web, HttpRequest};
use actix_web::{delete, get, post, put, Responder};
use actix_web::{web::Data, App, HttpResponse, HttpServer};
use actix_web::web::Json;
use derive_more::Display;
use dotenv::dotenv;
use hyper::Headers;
use hyper::header::{Authorization, Bearer};
use serde::{Deserialize, Serialize};
use sqlx::{self};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use pwhash::bcrypt;
use rand::distributions::{Alphanumeric, DistString};
use sqlx::postgres::PgSeverity::Error;

mod appstate;
use appstate::AppState;
use crate::MyError::InternalError;

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Auths {
    user_id: i32,
    user_token: String
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Users {
    user_id: i32,
    user_name: String,
    user_password: String,
    user_profession: String
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Todos {
    todo_id: i32,
    user_id: i32,
    description: String,
    todo_date: String,
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = Data::new(
        AppState {  
            db:
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Error building a connection pool")}
    );
    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .service(register)
            .service(login)
            .service(create)
            .service(get_todos_by_user_id)
            .service(get_todo_by_todo_id)
            .service(modify_by_todo_id)
            .service(delete_by_todo_id)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}


#[post("/register")]
pub async fn register(state: Data<AppState>, user: web::Json<Users>) -> impl Responder {

    let user =user.into_inner();
    let h_pwd =bcrypt::hash(&user.user_password.to_string()).unwrap();

    match sqlx::query_as!(Users,
        "INSERT INTO users (user_name, user_password, user_profession) VALUES ($1, $2, $3) RETURNING user_id, user_name, user_password, user_profession",
        user.user_name.to_string(), h_pwd, user.user_profession.to_string()
    )
        .fetch_one(&state.db)
        .await
    {
        Ok(user) => {
            HttpResponse::Ok().json(user)},
        Err(y) => {
            println!("{:?}",y);
            HttpResponse::InternalServerError().json("Failed to create user article")}
    }

}

#[post("/login")]
async fn login(state: Data<AppState>, user: web::Json<Users>) -> impl Responder {

    let table_user = sqlx::query_as!(
        Users, "select * from users where user_name =$1", user.user_name.to_string()
    )
        .fetch_one(&state.db).await;

    if bcrypt::verify(user.user_password.to_string(), table_user.unwrap().user_password.as_str()){

        match sqlx::query_as!(Users, "SELECT * FROM users WHERE user_name=$1", user.user_name.to_string()
        )
        .fetch_one(&state.db)
        .await
        {
            Ok(user) => {
                let user_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
                let mut headers = Headers::new();
                headers.set(
                   Authorization(
                       Bearer {
                           token: user_token.to_owned()
                       }
                   )
                );

                let authenticated_token = sqlx::query_as!(
                    Auths,"Insert into auths (user_id, user_token) VALUES ($1, $2)", user.user_id, user_token
                )
                .fetch_one(&state.db)
                .await;

                HttpResponse::Ok()
                .content_type(ContentType::plaintext())
                .insert_header(("Authorization", user_token))
                .body(serde_json::to_string(&user).unwrap())

            },
                Err(y) => {
                    println!("{:?}",y);
                    HttpResponse::InternalServerError().json("Failed to find USER in Database")}
                }
                
    } else {
        HttpResponse::InternalServerError().json("User password and username does not match")
    }
}
            
            
#[post("/todo")]
async fn create(state: Data<AppState>, todo: web::Json<Todos>, req:HttpRequest) -> impl Responder {

    let bearer = req.headers().get("Authorization").unwrap().to_str().unwrap().to_string();
    println!("{bearer}");
    let x=&bearer[7..];

    let b_user_id = sqlx::query_as!(Auths, "SELECT * FROM auths WHERE user_token=$1", x)
    .fetch_one(&state.db)
    .await;
    let b_id = b_user_id.unwrap().user_id;

    match sqlx::query_as!( Todos,
        "INSERT INTO todos (user_id, description, todo_date) VALUES ($1, $2, $3) RETURNING todo_id, user_id, description, todo_date",
        b_id, todo.description, todo.todo_date
    )
        .fetch_one(&state.db)
        .await
    {
        Ok(todo) => {
            HttpResponse::Ok().json(todo)},
        Err(y) => {
            println!("{:?}",y);
            HttpResponse::InternalServerError().json("Failed to create user article")}
    }
}

#[get("/todouser")]
async fn get_todos_by_user_id(state: Data<AppState>, req:HttpRequest) -> impl Responder {

    let bearer = req.headers().get("Authorization").unwrap().to_str().unwrap().to_string();
    let x=&bearer[7..];
    let b_user_id = sqlx::query_as!(Auths, "SELECT * FROM auths WHERE user_token=$1", x)
    .fetch_one(&state.db)
    .await;
    let b_user_id = b_user_id.unwrap().user_id;

    match sqlx::query_as!(Todos,"SELECT * FROM todos WHERE user_id=$1", b_user_id)
        .fetch_all(&state.db)
        .await
    {
        Ok(todos) => HttpResponse::Ok().json(todos),
        Err(_) => HttpResponse::NotFound().json("No Todos found"),
    }
}

#[get("/todo/{id}")]
async fn get_todo_by_todo_id(state: Data<AppState>, id: web::Path<i32>, req:HttpRequest) -> impl Responder {

    let id=id.into_inner();
    let bearer = req.headers().get("Authorization").unwrap().to_str().unwrap().to_owned();
    let x=&bearer[7..];
    let b_user_id = sqlx::query_as!(Auths, "SELECT * FROM auths WHERE user_token=$1", x)
        .fetch_one(&state.db)
        .await;
    let b_user_id = b_user_id.unwrap().user_id;

    let req_row = sqlx::query_as!(Todos, "SELECT * FROM todos WHERE todo_id=$1", id)
        .fetch_one(&state.db)
        .await;
    let req_id = req_row.unwrap().user_id;

    if req_id==b_user_id {
    match sqlx::query_as::<_, Todos>(
        "SELECT * FROM todos WHERE todo_id=$1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    {
        Ok(todo) => HttpResponse::Ok().json(todo),
        Err(_) => HttpResponse::InternalServerError().json("Failed to get user todo"),
    }
    }else{
        HttpResponse::InternalServerError().json("not owner of relevant todo_id to update user todo")
    }
}

#[put("todo/{id}")]
async fn modify_by_todo_id( state: Data<AppState>, id: web::Path<i32>, todo: web::Json<Todos>, req:HttpRequest) -> impl Responder {
    let id=id.into_inner();
    let bearer = req.headers().get("Authorization").unwrap().to_str().unwrap().to_string();
    let x=&bearer[7..];
    let b_user_id = sqlx::query_as!(Auths, "SELECT * FROM auths WHERE user_token=$1", x)
    .fetch_one(&state.db)
    .await;
    let b_user_id = b_user_id.unwrap().user_id;

    let req_row = sqlx::query_as!(Todos, "SELECT * FROM todos WHERE todo_id=$1", id)
    .fetch_one(&state.db)
    .await;
    let req_id = req_row.unwrap().user_id;

    if req_id==b_user_id {
        let todo = todo.into_inner();
        match sqlx::query_as!( Todos, "UPDATE todos SET description=$1, todo_date=$2 WHERE
        todo_id=$3 RETURNING todo_id, user_id, description, todo_date", 
        &todo.description, &todo.todo_date, id
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(todo) => HttpResponse::Ok().json(todo),
        Err(_) => HttpResponse::InternalServerError().json("Failed to update user todo"),
    }
    }else{
    HttpResponse::InternalServerError().json("not owner of relevant todo_id to update user todo")   
    }
}

#[delete("todo/{id}")]
async fn delete_by_todo_id(state: Data<AppState>, id: web::Path<i32>, req:HttpRequest,) -> impl Responder {

    let id=id.into_inner();
    let bearer = req.headers().get("Authorization").unwrap().to_str().unwrap();
    let x=&bearer[7..];
    let b_user_id = sqlx::query_as!(Auths, "SELECT * FROM auths WHERE user_token=$1", x)
        .fetch_one(&state.db)
        .await;
    let b_user_id = b_user_id.unwrap().user_id;

    let req_row = sqlx::query_as!(Todos, "SELECT * FROM todos WHERE todo_id=$1", id)
        .fetch_one(&state.db)
        .await;
    let req_id = req_row.unwrap().user_id;


    if req_id==b_user_id {
    match sqlx::query_as::<_, Todos>(
        "DELETE FROM todos WHERE todo_id=$1 RETURNING todo_id, user_id, description, date",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    {
        Ok(todo) =>
            HttpResponse::Ok().json(todo),
        Err(_) => HttpResponse::InternalServerError().json("Failed to create user article"),
    }
    }else{
        HttpResponse::InternalServerError().json("not owner of relevant todo_id to update user todo")
    }
}

#[derive(Debug,Display)]
enum MyError {
    InternalError,
    NoContent,
}

impl actix_web::error::ResponseError for MyError {}

impl From<sqlx::Error> for MyError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Self::NoContent,
            _ => InternalError
        }
    }
}

/* TODO
1) Middleware
2) Error handling:
    a) Define enum of your errors
    b) Implement Response error for your errors
    c) Implement From<sqlx::Error> for your error so that sqlx errors can be automatically converted to your errors
    d) Use question mark operator where you can
    e) Try to use less unwraps
    f) Try to clone less
3) Don't use select * in code.
 */

