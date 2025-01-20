import gradio as gr
import requests
import numpy as np
import soundfile as sf
import io
import time

# 可用的声音风格（带性别标注）
VOICES = {
    "af": "AF（女声）",
    "af_nicole": "Nicole（女声）",
    "af_bella": "Bella（女声）",
    "am_adam": "Adam（男声）",
    "af_sky": "Sky（女声）",
    "af_sarah": "Sarah（女声）",
    "bf_emma": "Emma（女声）",
    "bf_isabella": "Isabella（女声）",
    "bm_george": "George（男声）",
    "bm_lewis": "Lewis（男声）",
    "am_michael": "Michael（男声）"
}

# 支持的语言
LANGUAGES = {
    "en-us": "英语（完整支持）",
    "zh-cn": "中文（部分支持）",
    "ja-jp": "日语（部分支持）",
    "de-de": "德语（部分支持）"
}

def text_to_speech(text, voice1, language="en-us", voice2=None, mix_ratio=None):
    """
    调用 TTS API 将文本转换为语音
    """
    # 获取实际的声音ID
    voice1_id = [k for k, v in VOICES.items() if v == voice1][0]
    
    # 构建声音风格
    if voice2 and mix_ratio is not None:
        voice2_id = [k for k, v in VOICES.items() if v == voice2][0]
        # 使用整数格式（不带小数点）
        ratio1 = int(mix_ratio / 10)  # 40% -> 4
        ratio2 = int((100 - mix_ratio) / 10)  # 60% -> 6
        voice = f"{voice1_id}.{ratio1}+{voice2_id}.{ratio2}"
        print(f"使用声音配置: {voice} (比例: {ratio1}, {ratio2})")
    else:
        voice = voice1_id
        print(f"使用声音: {voice}")
    
    try:
        response = requests.post(
            "http://localhost:3000/v1/audio/speech",
            json={
                "model": "kokoro-v0_19",
                "input": text,
                "voice": voice,
                "language": language  # 添加语言参数
            }
        )
        
        if response.status_code == 200:
            # 读取生成的音频文件
            audio_path = "tmp/output.wav"
            
            # 增加重试机制，最多等待5秒
            max_retries = 10
            retry_interval = 0.5
            
            for _ in range(max_retries):
                try:
                    # 等待文件写入完成
                    time.sleep(retry_interval)
                    audio, sr = sf.read(audio_path)
                    
                    # 打印音频信息用于调试
                    print(f"音频信息: 形状={audio.shape}, 类型={audio.dtype}, 采样率={sr}")
                    
                    # 如果是双声道，只取第一个声道
                    if len(audio.shape) > 1 and audio.shape[1] > 1:
                        audio = audio[:, 0]
                    
                    return (sr, audio)
                except Exception as e:
                    print(f"尝试读取音频文件失败: {e}, 重试中...")
                    continue
            
            print("无法读取音频文件，已达到最大重试次数")
            return None
        else:
            print(f"API 错误: {response.status_code}")
            return None
            
    except Exception as e:
        print(f"Error: {e}")
        return None

def create_ui():
    with gr.Blocks(title="Kokoro TTS 测试界面") as demo:
        gr.Markdown("""
        # Kokoro TTS 测试界面
        
        这是一个用于测试 Kokoro TTS 的界面。你可以：
        1. 输入要转换的文本
        2. 选择语言（支持英语、中文、日语、德语）
        3. 选择单个声音，或选择两种声音进行混合
        4. 如果选择混合模式，可以调整混合比例
        
        可用声音说明：
        - AF/Nicole/Bella/Sky/Sarah/Emma/Isabella：女声
        - Adam/George/Lewis/Michael：男声
        """)
        
        with gr.Row():
            with gr.Column():
                text_input = gr.Textbox(
                    label="输入文本",
                    placeholder="请输入要转换的文本...",
                    lines=5
                )
                
                language = gr.Dropdown(
                    choices=list(LANGUAGES.values()),
                    value="英语（完整支持）",
                    label="选择语言"
                )
                
                mode = gr.Radio(
                    choices=["单声音", "混合声音"],
                    value="单声音",
                    label="模式选择"
                )
                
                with gr.Row():
                    voice1 = gr.Dropdown(
                        choices=list(VOICES.values()),
                        value="Sarah（女声）",
                        label="声音 1"
                    )
                    
                with gr.Row(visible=False) as mix_controls:
                    voice2 = gr.Dropdown(
                        choices=list(VOICES.values()),
                        value="Nicole（女声）",
                        label="声音 2"
                    )
                    mix_ratio = gr.Slider(
                        minimum=0,
                        maximum=100,
                        value=40,
                        step=10,
                        label="声音 1 的混合比例 (%)"
                    )
                
                submit_btn = gr.Button("生成语音")
            
            with gr.Column():
                audio_output = gr.Audio(
                    label="生成的语音",
                    type="numpy"
                )
                
                status_text = gr.Textbox(
                    label="状态信息",
                    interactive=False
                )
        
        def update_mode(choice):
            return gr.Row.update(visible=(choice == "混合声音"))
        
        mode.change(
            fn=update_mode,
            inputs=[mode],
            outputs=[mix_controls]
        )
                
        def process_tts(text, language, mode, voice1, voice2=None, mix_ratio=None):
            try:
                # 获取实际的语言代码
                lang_code = [k for k, v in LANGUAGES.items() if v == language][0]
                if mode == "单声音":
                    result = text_to_speech(text, voice1, lang_code)
                else:
                    result = text_to_speech(text, voice1, lang_code, voice2, mix_ratio)
                    
                if result is not None:
                    return result, "生成成功！"
                return None, "生成失败，请查看控制台输出。"
            except Exception as e:
                return None, f"错误：{str(e)}"
                
        submit_btn.click(
            fn=process_tts,
            inputs=[text_input, language, mode, voice1, voice2, mix_ratio],
            outputs=[audio_output, status_text]
        )
    
    return demo

if __name__ == "__main__":
    demo = create_ui()
    demo.launch(share=True) 