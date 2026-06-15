import {footer_tpl} from '../view/footer.ts';

export const footer_init = ()=>{
    const footer_node = document.querySelector('.footer');
    if(footer_node){
        footer_node.innerHTML = footer_tpl;
    }
}
