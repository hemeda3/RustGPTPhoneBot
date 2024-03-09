# RustGPTPhoneBot (WIP)
###### Rust-based phone bot managed by (GPT-3) call-center for E-Commerce using local mobile numbers  (under construction but with enough effort you can make it work)

<center>

![img.png](img.jpeg)

</center>


Functionality
-------------
- **Interactive Conversations**: Engages users with context-aware responses, guided by pre-defined scripts and user inputs.
- **Environment Variables Management**: Handles sensitive data like API keys securely through environment variables.
- **Error Handling**: Robustly manages potential errors, ensuring uninterrupted service.
-   **Call Management**: Handles call events like initiation, answering, and hangup.
-   **Audio Streaming**: Manages real-time audio data streaming via WebSockets.
-   **Speech Recognition and Synthesis:** Converts speech to text and synthesizes text to speech.
-   **AI-Driven Interaction**: Utilizes GPT models for generating human-like responses.
-   **Error Handling and Logging**: Robust error management and detailed logging for diagnostics. 
-   **Telephony Integration**: Handles call events, including initiation, answering, and hangup.

The bot exemplifies a modern approach to telecommunication, combining cloud-based services with advanced AI models, and highlights Rust's capability in handling concurrent, real-time communication tasks.

## Why

* I want call center controlled by GPT (AI) 24/7
* This GPT 3 Phonebot answers any FAQS question 
* It can even interact with customers like search for products or gather feedback with help from GCP + Azure + Eleven Labs ( for speech recognition + Speech Synthesis).
* In future  the PhoneBot will be able to add to cart and place order 

## Overview
- **RustGPTPhoneBot**: A Rust-based phone bot leveraging GPT-3 technology.

- **E-Commerce Assistance**: Designed to aid customers of Shoppers Drug Mart, Instacart, Walmart, and similar businesses.

- **Local Mobile Number Integration**: Operates over local mobile networks for real-time assistance.

- **AI-Driven Chat Capabilities**: Provides advanced, AI-powered chat support to users.

- **Voicemail Control Feature**: coming soon, the ability to manage and control your voicemail systems.

## Key Features
- **FAQ Assistance**: Offers detailed answers to frequently asked questions about Shoppers Drug Mart.
- **CSAT Survey Implementation**: Conducts customer satisfaction surveys using Structured Synthetic Markup Language (SSML) for enhanced interaction.
- **ChatGPT Integration**: Leverages OpenAI's ChatGPT for natural language processing and response generation.
- **Real-time Interaction**: Handles customer queries in real-time, offering a seamless user experience.

## Technical Stack
- **Rust**: Core programming language for performance and safety.
- **Warp**: For RESTful API creation and handling HTTP requests.
- **Tokio**: Asynchronous runtime for non-blocking operation.
- **Serde**: For serialization and deserialization of data.
- **Lazy Static**: For efficient memory management and performance optimization.
- 
## Telecommunication and Speech Processing
- **Telnyx API:** For call control (hangup, send DTMF, call info).
- **Cognitive Speech Services (Azure)**: Speech recognition and synthesis.
- **Google Cloud Speech-to-Text (GCP)**: For additional speech recognition capabilities.

## Third-Party Integrations
  - **Telnyx API**: For telephony services.
  - **OpenAI (ChatGPT)**: For AI-driven chat responses.
  - **Redis**: For data caching and session management.
  - **Microsoft Cognitive Services**: For speech recognition and synthesis.
  - **Eleven Labs API**: For additional voice synthesis capabilities.

## Usage
- **Customer Interaction**: Assists customers with inquiries related to Shoppers Drug Mart products, orders, and services.
- **Survey Execution**: Collects customer feedback through structured surveys, using SSML for an engaging experience.

## Deployment and Configuration
- Utilizes environmental variables for secure configuration of API keys and endpoints.
- Configurable for different operational modes, including standard and streaming chat interactions.
- Docker file ready for GCP deployment
- for MAC dev you need install MicrosoftCognitiveServicesSpeech.xcframework https://aka.ms/csspeech/macosbinary 

## Key Environment Variables
- `TELNYX_API_KEY`: Key for Telnyx API integration.
- `TELNYX_API_URL`: Base URL for Telnyx API calls.
- `REDIS_URI`: URI for Redis server connection.
- `REDIS_PORT`: Port number for Redis server.
- `REDIS_PASSWORD`: Password for Redis server access.
- `OPENAI_KEY`: API key for OpenAI's services.
- `MSServiceRegion`: Microsoft Cognitive Services region.
- `MSSubscriptionKey`: Subscription key for Microsoft Cognitive Services.
- `RUST_LOG`: Logging level for the Rust application.
- `RUST_BACKTRACE`: Enables backtracing for debugging in Rust.
- export MSServiceRegion=eastus
- export DYLD_FALLBACK_FRAMEWORK_PATH=~/speechsdk/MicrosoftCognitiveServicesSpeech.xcframework/macos-arm64_x86_64 // only macos for linux it's set in docker
- export MACOS_SPEECHSDK_ROOT=/Users/[YOUR USER NAME ]/speechsdk

## Initialization
These variables are set at the beginning of the `main` function, ensuring all services and APIs are correctly configured before the server starts.

## Logging and Debugging
The application uses `env_logger` for initializing the logging framework, controlled by the `RUST_LOG` and `RUST_BACKTRACE` variables.

## Server Routes
The server routes for handling webhook and WebSocket connections are defined using Warp, with the WebSocket route handling real-time communication based on the `call_id`.

## Execution
The server is executed on the specified `PORT`, defaulting to 8080, making it compatible with various hosting environments, including cloud platforms.
