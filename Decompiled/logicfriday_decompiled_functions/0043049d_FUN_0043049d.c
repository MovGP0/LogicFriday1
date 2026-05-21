/* 0043049d FUN_0043049d */

void FUN_0043049d(HWND param_1,int *param_2,LPCSCROLLINFO param_3,LPCSCROLLINFO param_4,int param_5,
                 int param_6)

{
  int iVar1;
  tagRECT local_14;
  
  GetClientRect(param_1,&local_14);
  param_3->fMask = 0xf;
  param_4->fMask = 0xf;
  param_3->nMin = 0;
  param_3->nMax = param_2[2] - *param_2;
  iVar1 = FUN_0043f3b8(*param_2);
  param_3->nPos = iVar1 + param_5;
  param_3->nPage = local_14.right + 1;
  if ((int)(param_3->nMax - param_3->nPage) < param_3->nPos) {
    param_3->nPage = param_3->nMax - param_3->nPos;
  }
  param_4->nMin = 0;
  param_4->nMax = param_2[3] - param_2[1];
  iVar1 = FUN_0043f3b8(param_2[1]);
  param_4->nPos = iVar1 + param_6;
  param_4->nPage = local_14.bottom + 1;
  if ((int)(param_4->nMax - param_4->nPage) < param_4->nPos) {
    param_4->nPage = param_4->nMax - param_4->nPos;
  }
  SetScrollInfo(param_1,0,param_3,1);
  SetScrollInfo(param_1,1,param_4,1);
  return;
}
